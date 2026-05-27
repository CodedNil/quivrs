use crate::{
    server::{
        database::LabelScore,
        embeddings::{
            EMBEDDING_TITLE_REPEAT, article_text, best_label, binary_label_score,
            generate_embeddings, label_definitions,
        },
    },
    shared::{Category, PendingSource},
};
use anyhow::{Context, Result};

struct TestCase {
    title: &'static str,
    summary: &'static str,
    category: Category,
    sentiment: f32,
    importance: f32,
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        title: "Flowers laid as teenage girl found dead in water at Kingsbury country park",
        summary: "The girl's body has been recovered from the water following concerns for her welfare, police say.",
        category: Category::Law,
        sentiment: 0.0,
        importance: 0.3,
    },
    TestCase {
        title: "Teenage patients 'ignored' before fatal NHS trust failures",
        summary: "Mental health patients say nobody listened to their concerns about a north-east England trust.",
        category: Category::Health,
        sentiment: 0.0,
        importance: 0.3,
    },
    TestCase {
        title: "Woman warned to prepare for jail after XL Bully kills Shrewsbury teenager",
        summary: "Teenager Morgan Dorsett, 19, suffered fatal bites to her neck in the XL Bully attack.",
        category: Category::Law,
        sentiment: 0.0,
        importance: 0.1,
    },
    TestCase {
        title: "Energy bills to rise for millions as impact of Iran war hits",
        summary: "A household using a typical amount of energy will pay £221 a year more, under the regulator's new price cap.",
        category: Category::Lifestyle,
        sentiment: 0.1,
        importance: 0.8,
    },
    TestCase {
        title: "Why temperature records are being not only broken but smashed",
        summary: "The combined effects of a heat dome and climate change have brought extreme warmth to western Europe.",
        category: Category::Nature,
        sentiment: 0.1,
        importance: 0.9,
    },
    TestCase {
        title: "A rare ancient rainforest set to come back to life",
        summary: "Ulster Wildlife takes on 100 year restoration project of ancient rainforest.",
        category: Category::Nature,
        sentiment: 0.7,
        importance: 0.4,
    },
    TestCase {
        title: "River Waveney land purchased to save it from risk of development",
        summary: "A council hopes to enhance wildlife in the area and improve public access.",
        category: Category::Nature,
        sentiment: 0.8,
        importance: 0.5,
    },
    TestCase {
        title: "Scottish first-time buyers offered up to £10,000 interest-free loans",
        summary: "The Scottish government has launched the First Homes Fund for those looking to get on the property ladder.",
        category: Category::Politics,
        sentiment: 0.9,
        importance: 0.7,
    },
    TestCase {
        title: "The Saatva mattress I recommend to everyone is $400 off in a huge last-chance Memorial Day sale",
        summary: "The Saatva Classic is our number one rated mattress.",
        category: Category::Lifestyle,
        sentiment: 0.5,
        importance: 0.0,
    },
];

#[test]
fn category_classification() -> Result<()> {
    let mut failures = Vec::new();

    for (case, scores) in TEST_CASES.iter().zip(label_scores("category")?) {
        let category = best_label::<Category>(&scores, "category")?;
        if category != case.category {
            failures.push(format!(
                "{category:<10} expected {:<10} Title: {}",
                case.category, case.title
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "\n{} category cases misclassified:\n\nactual     expected   title\n{}",
        failures.len(),
        failures.join("\n")
    );

    Ok(())
}

#[test]
fn sentiment_scores() -> Result<()> {
    assert_binary_scores("sentiment", "positive", |case| case.sentiment)
}

#[test]
fn importance_scores() -> Result<()> {
    assert_binary_scores("importance", "important", |case| case.importance)
}

fn label_scores(group: &str) -> Result<Vec<Vec<LabelScore>>> {
    let definitions = label_definitions()
        .into_iter()
        .filter(|d| d.label_group == group)
        .collect::<Vec<_>>();

    let texts = definitions
        .iter()
        .map(|d| d.text.to_string())
        .chain(TEST_CASES.iter().map(|case| {
            article_text(
                &PendingSource {
                    title: case.title.to_string(),
                    summary: case.summary.to_string(),
                    ..Default::default()
                },
                EMBEDDING_TITLE_REPEAT,
            )
        }))
        .collect::<Vec<_>>();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create Tokio runtime")?;

    let embeddings = rt.block_on(generate_embeddings(&texts))?;
    let (label_embeddings, article_embeddings) = embeddings.split_at(definitions.len());

    Ok(article_embeddings
        .iter()
        .map(|article_embedding| {
            definitions
                .iter()
                .zip(label_embeddings)
                .map(|(definition, label_embedding)| LabelScore {
                    label_group: definition.label_group.to_string(),
                    label_value: definition.label_value.clone(),
                    similarity: article_embedding
                        .iter()
                        .zip(label_embedding)
                        .map(|(l, r)| l * r)
                        .sum(),
                })
                .collect()
        })
        .collect())
}

fn assert_binary_scores(
    label_group: &str,
    positive_value: &str,
    expected_score: impl Fn(&TestCase) -> f32,
) -> Result<()> {
    let mut failures = Vec::new();

    for (case, scores) in TEST_CASES.iter().zip(label_scores(label_group)?) {
        let score = binary_label_score(&scores, label_group, positive_value)?;
        let expected = expected_score(case);
        let expected_min = expected.clamp(0.1, 0.9) - 0.1;
        let expected_max = expected.clamp(0.1, 0.9) + 0.1;

        if !(expected_min..=expected_max).contains(&score) {
            failures.push(format!(
                "{score:>5.3}  {expected_min:.2}..={expected_max:.2}  Title: {}",
                case.title
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "\n{} {label_group} cases outside expected range (±0.1):\n\nscore  expected      title\n{}",
        failures.len(),
        failures.join("\n")
    );

    Ok(())
}
