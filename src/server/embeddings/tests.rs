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
        title: "Aldi announces £5bn investment in UK food production",
        summary: "Aldi has pledged over £5 billion in long-term supplier agreements, in a move which will enable its suppliers to invest in future operations.",
        category: Category::Business,
        sentiment: 0.8,
        importance: 0.4,
    },
    TestCase {
        title: "Scottish first-time buyers offered up to £10,000 interest-free loans",
        summary: "The Scottish government has launched the First Homes Fund for those looking to get on the property ladder.",
        category: Category::Politics,
        sentiment: 0.9,
        importance: 0.7,
    },
    TestCase {
        title: "Reform UK civil service plan 'would sack more planning officers than exist'",
        summary: "Analysis of party's proposed cuts also suggests it would get rid of two-thirds of psychologists who support prison staff.",
        category: Category::Politics,
        sentiment: 0.2,
        importance: 0.6,
    },
    TestCase {
        title: "Flowers laid as teenage girl found dead in water at Kingsbury country park",
        summary: "The girl's body has been recovered from the water following concerns for her welfare, police say.",
        category: Category::Law,
        sentiment: 0.0,
        importance: 0.3,
    },
    TestCase {
        title: "Woman warned to prepare for jail after XL Bully kills Shrewsbury teenager",
        summary: "Teenager Morgan Dorsett, 19, suffered fatal bites to her neck in the XL Bully attack.",
        category: Category::Law,
        sentiment: 0.0,
        importance: 0.2,
    },
    TestCase {
        title: "Teenage patients 'ignored' before fatal NHS trust failures",
        summary: "Mental health patients say nobody listened to their concerns about a north-east England trust.",
        category: Category::Health,
        sentiment: 0.0,
        importance: 0.5,
    },
    TestCase {
        title: "Former US Attorney General Pam Bondi diagnosed with cancer",
        summary: "Bondi, who was diagnosed shortly after being ousted in April, is also joining the White House's new advisory council on AI.",
        category: Category::Health,
        sentiment: 0.1,
        importance: 0.4,
    },
    TestCase {
        title: "How to watch 'A Good Girl's Guide To Murder' season 2 online - stream the hit crime-thriller series for FREE",
        summary: "Pip and Ravi are back for more mystery-solving with season 2 of A Good Girl's Guide To Murder and you can watch along for free.",
        category: Category::Culture,
        sentiment: 0.7,
        importance: 0.0,
    },
    TestCase {
        title: "Energy bills to rise for millions as impact of Iran war hits",
        summary: "A household using a typical amount of energy will pay £221 a year more, under the regulator's new price cap.",
        category: Category::Lifestyle,
        sentiment: 0.0,
        importance: 0.8,
    },
    TestCase {
        title: "The Saatva mattress I recommend to everyone is $400 off in a huge last-chance Memorial Day sale",
        summary: "The Saatva Classic is our number one rated mattress.",
        category: Category::Lifestyle,
        sentiment: 0.5,
        importance: 0.0,
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
        title: "Dead sharks and fish found washed up on Welsh beaches",
        summary: "The dead fish, at Saundersfoot and Pembrey beaches, were probably part of a discarded catch.",
        category: Category::Nature,
        sentiment: 0.1,
        importance: 0.3,
    },
    TestCase {
        title: "space-tree: Workspace Management Trees in Emacs",
        summary: "space-tree is a tree-based workspace manager for Emacs. Workspaces are a battle-tested UX concept across operating systems, but in Emacs and most OSes alike.",
        category: Category::Software,
        sentiment: 0.6,
        importance: 0.0,
    },
    TestCase {
        title: "A new Register Allocator for ZJIT",
        summary: "We recently landed a new register allocator in ZJIT. Learn about lifetimes, interference graphs, and linear scan - and why a global allocator unlocks features like method inlining.",
        category: Category::Software,
        sentiment: 0.6,
        importance: 0.1,
    },
    TestCase {
        title: "French Open 2026 results: Tamara Korpatsch and Wang Xinyu have heated exchange and avoid handshake in Roland Garros row",
        summary: "Germany's Tamara Korpatsch says it would be embarrassing if she cheated against China's Wang Xinyu following their heated exchange at the French Open.",
        category: Category::Sports,
        sentiment: 0.2,
        importance: 0.2,
    },
    TestCase {
        title: "Sony is offering up to 50 percent off some of our favorite PS5 games",
        summary: "Sony is discounting a host of PS5 games as part of its latest Days of Play promo. You can also save on Govee's Lightwall and the Switch port of Red Dead Redemption.",
        category: Category::Gaming,
        sentiment: 0.8,
        importance: 0.1,
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
