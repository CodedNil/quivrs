use crate::{
    server::{
        database::LabelScore,
        embeddings::{
            EMBEDDING_TITLE_REPEAT, article_text, best_label, binary_label_score,
            generate_embeddings, label_definitions,
        },
    },
    shared::{Category, PendingSource, Region},
};
use anyhow::{Context, Result};

// Label-writing guide for embeddinggemma-300m:
//
// 1. Keyword Density Over Prose
//    - Rule: Favor dense clusters of distinct keywords separated by colons and commas. Avoid full sentences.
//    - Why: Fillers like "This category represents stories about..." dilute the vector space.
//      Use punctuation (colons, commas) to chain concepts together cleanly.
//    - Example: Prefer "Macroeconomic strain: inflation, rising energy bills, financial hardship" over
//      "Articles discussing how the economy is struggling and forcing families to pay more for their household bills."
//
// 2. Strict Prohibition of Negation
//    - Rule: Never use logical inversions or negative modifiers (e.g., "not", "non", "without", "except").
//      Do not define a low-weight category by contrasting it against high-weight concepts.
//      For example, never use the phrase 'non-emergency' or 'non-violent' in an unimportant/positive bucket.
//    - Why: Bi-encoders match tokens based on presence, not logic. "Not technology" heavily attracts tech articles.
//      The high-weight tokens ('emergency', 'violent') will bleed into the vector space regardless of the prefix.
//    - Fix: Describe what fills the void instead (e.g., use "Routine consumer lifestyle" instead of "Non-important news").
//      "routine orbital space operations" instead of "non-emergency satellite launches"
//
// 3. Absolute Conceptual Abstraction (No Overfitting or Proper Nouns)
//    - Rule: Never copy literal strings from failing test cases, specific brands, proper nouns, or fleeting fixtures.
//    - Why: Including "Google Willow" shifts the vector toward "Big Tech corporations" rather than anchoring it to "Quantum Physics".
//      It warps the embedding space for related topics and breaks generality.
//    - Fix: Strengthen the general, conceptual parent synonyms (e.g., swap "Ferrari" for "Next-generation automotive design").
//      If a local tragedy scores too high on importance, don't name the tragedy; strengthen the explicit concept of local scale.
//      Use "drama series" instead of "crime dramas" so the model doesn't latch onto the word crime.
//
// 4. Anchor Macro vs. Micro "Scale" for Importance
//    - Rule: Explicitly use systemic words (widespread, national, macro) vs. localized words (isolated, individual, niche, minor).
//    - Why: Embedding models naturally confuse high emotional weight (e.g., an individual death) with high national importance.
//      You must mathematically anchor the explicit scope of the event.
//
// 5. Global Symmetry & Structural Balance
//    - Rule: Keep opposing buckets perfectly mirrored in tone, breadth, and level of abstraction.
//    - Why: If one group uses academic terminology and the other uses casual prose, the model will classify based on
//      writing style rather than actual sentiment or importance.
//
// 6. Multi-Label Redundancy (The Safety Net)
//    - Rule: Ensure high-priority target concepts are explicitly mentioned across at least one label string in the group.
//    - Why: Category classification picks the closest single label in vector space. Redundant synonyms ensure a tight
//      mathematical distance for edge cases.

const PLUS_MINUS_MARGIN: f32 = 0.15;

struct TestCase {
    title: &'static str,
    summary: &'static str,
    category: Category,
    region: Region,
    sentiment: f32,
    importance: f32,
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        title: "Aldi announces £5bn investment in UK food production",
        summary: "Aldi has pledged over £5 billion in long-term supplier agreements, in a move which will enable its suppliers to invest in future operations.",
        category: Category::Business,
        region: Region::UnitedKingdom,
        sentiment: 0.7,
        importance: 0.4,
    },
    TestCase {
        title: "Manchester University to offer work placements to all undergraduates",
        summary: "Russell Group university promises students, from chemical engineering to classics, ‘meaningful real-world experience’",
        category: Category::Business,
        region: Region::England,
        sentiment: 0.8,
        importance: 0.6,
    },
    TestCase {
        title: "Scottish first-time buyers offered up to £10,000 interest-free loans",
        summary: "The Scottish government has launched the First Homes Fund for those looking to get on the property ladder.",
        category: Category::Politics,
        region: Region::Scotland,
        sentiment: 0.8,
        importance: 0.7,
    },
    TestCase {
        title: "Reform UK civil service plan 'would sack more planning officers than exist'",
        summary: "Analysis of party's proposed cuts also suggests it would get rid of two-thirds of psychologists who support prison staff.",
        category: Category::Politics,
        region: Region::UnitedKingdom,
        sentiment: 0.3,
        importance: 0.5,
    },
    TestCase {
        title: "Jill Biden says she thought Joe Biden was having a stroke during 2024 debate",
        summary: "The former US first lady told CBS News that she was frightened by Joe Biden's performance against Donald Trump.",
        category: Category::Politics,
        region: Region::UnitedStates,
        sentiment: 0.3,
        importance: 0.6,
    },
    TestCase {
        title: "Russia is targeting UK’s infrastructure and democracy, GCHQ head to say",
        summary: "Anne Keast-Butler will also warn of narrowing window to stay ahead of China in ‘new era of radical uncertainty’",
        category: Category::Politics,
        region: Region::UnitedKingdom,
        sentiment: 0.2,
        importance: 0.8,
    },
    TestCase {
        title: "Flowers laid as teenage girl found dead in water at Kingsbury country park",
        summary: "The girl's body has been recovered from the water following concerns for her welfare, police say.",
        category: Category::Law,
        region: Region::England,
        sentiment: 0.2,
        importance: 0.3,
    },
    TestCase {
        title: "Woman warned to prepare for jail after XL Bully kills Shrewsbury teenager",
        summary: "Teenager Morgan Dorsett, 19, suffered fatal bites to her neck in the XL Bully attack.",
        category: Category::Law,
        region: Region::England,
        sentiment: 0.1,
        importance: 0.3,
    },
    TestCase {
        title: "Australia sues 3M over 'forever chemicals' in firefighting foam",
        summary: "The A$2bn case, which centres on contamination at defence sites, is the largest ever brought by the government.",
        category: Category::Law,
        region: Region::Oceania,
        sentiment: 0.3,
        importance: 0.5,
    },
    TestCase {
        title: "Men arrested after alleged sexual assault of teens on beach.",
        summary: "Two men were arrested on Bournemouth Beach yesterday on suspicion of sexually assaulting two teenage girls.",
        category: Category::Law,
        region: Region::England,
        sentiment: 0.2,
        importance: 0.4,
    },
    TestCase {
        title: "Utumishi Girls school fire: Sixteen students killed in Kenya school blaze.",
        summary: "Search-and-rescue operations are ongoing, with the number of casualties yet to be confirmed",
        category: Category::Law,
        region: Region::SubSaharanAfrica,
        sentiment: 0.1,
        importance: 0.8,
    },
    TestCase {
        title: "Teenage patients 'ignored' before fatal NHS trust failures",
        summary: "Mental health patients say nobody listened to their concerns about a north-east England trust.",
        category: Category::Health,
        region: Region::England,
        sentiment: 0.0,
        importance: 0.5,
    },
    TestCase {
        title: "Former US Attorney General Pam Bondi diagnosed with cancer",
        summary: "Bondi, who was diagnosed shortly after being ousted in April, is also joining the White House's new advisory council on AI.",
        category: Category::Health,
        region: Region::UnitedStates,
        sentiment: 0.1,
        importance: 0.5,
    },
    TestCase {
        title: "How to watch 'A Good Girl's Guide To Murder' season 2 online - stream the hit crime-thriller series for FREE",
        summary: "Pip and Ravi are back for more mystery-solving with season 2 of A Good Girl's Guide To Murder and you can watch along for free.",
        category: Category::Culture,
        region: Region::Global,
        sentiment: 0.6,
        importance: 0.0,
    },
    TestCase {
        title: "Energy bills to rise for millions as impact of Iran war hits",
        summary: "A household using a typical amount of energy will pay £221 a year more, under the regulator's new price cap.",
        category: Category::Lifestyle,
        region: Region::UnitedKingdom,
        sentiment: 0.0,
        importance: 0.8,
    },
    TestCase {
        title: "The Saatva mattress I recommend to everyone is $400 off in a huge last-chance Memorial Day sale",
        summary: "The Saatva Classic is our number one rated mattress.",
        category: Category::Lifestyle,
        region: Region::UnitedStates,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Skechers walking and running shoes are still discounted after Memorial Day — 13 best deals with up to 48% off",
        summary: "Skechers walking and running shoes",
        category: Category::Lifestyle,
        region: Region::UnitedStates,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "All the news about Ferrari’s polarizing Luce EV",
        summary: "The first electric car from Ferrari is here, and the design of the Luce has fans saying it doesn’t look like a true Ferrari.",
        category: Category::Transport,
        region: Region::Global,
        sentiment: 0.7,
        importance: 0.4,
    },
    TestCase {
        title: "Why temperature records are being not only broken but smashed",
        summary: "The combined effects of a heat dome and climate change have brought extreme warmth to western Europe.",
        category: Category::Nature,
        region: Region::WesternEurope,
        sentiment: 0.2,
        importance: 0.8,
    },
    TestCase {
        title: "Tom's Guide to tomatoes",
        summary: "Everything you need to know about planting, growing, pruning, and harvesting tomatoes.",
        category: Category::Nature,
        region: Region::Global,
        sentiment: 0.6,
        importance: 0.2,
    },
    TestCase {
        title: "A rare ancient rainforest set to come back to life",
        summary: "Ulster Wildlife takes on 100 year restoration project of ancient rainforest.",
        category: Category::Nature,
        region: Region::Ireland,
        sentiment: 0.8,
        importance: 0.5,
    },
    TestCase {
        title: "River Waveney land purchased to save it from risk of development",
        summary: "A council hopes to enhance wildlife in the area and improve public access.",
        category: Category::Nature,
        region: Region::England,
        sentiment: 0.8,
        importance: 0.6,
    },
    TestCase {
        title: "Dead sharks and fish found washed up on Welsh beaches",
        summary: "The dead fish, at Saundersfoot and Pembrey beaches, were probably part of a discarded catch.",
        category: Category::Nature,
        region: Region::Wales,
        sentiment: 0.2,
        importance: 0.4,
    },
    TestCase {
        title: "Peebles river detectives part of network tackling pollution",
        summary: "Hundreds of volunteers survey Scotland's watercourses and report any issues of concern quickly",
        category: Category::Nature,
        region: Region::Scotland,
        sentiment: 0.8,
        importance: 0.5,
    },
    TestCase {
        title: "I put aluminum foil behind my router to fix my Wi-Fi dead zones — and it actually worked",
        summary: "Curved aluminum foil behind your router reflects Wi-Fi signals directly toward your dead zones, instead of letting them scatter",
        category: Category::Technology,
        region: Region::Global,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Oura Ring 5 could be unveiled this week with new design — here's why I'm concerned. Smart Rings.",
        summary: "A thinner design could have implications for battery life",
        category: Category::Technology,
        region: Region::Global,
        sentiment: 0.4,
        importance: 0.1,
    },
    TestCase {
        title: "The new Halide camera app launches with film looks and an upgraded photo editor.",
        summary: "Halide Mark III is now available for iPhones and iPads with an updated interface, new film looks, and an upgraded editor that can process third-party RAW files.",
        category: Category::Software,
        region: Region::Global,
        sentiment: 0.7,
        importance: 0.2,
    },
    TestCase {
        title: "space-tree: Workspace Management Trees in Emacs",
        summary: "space-tree is a tree-based workspace manager for Emacs. Workspaces are a battle-tested UX concept across operating systems, but in Emacs and most OSes alike.",
        category: Category::Software,
        region: Region::Global,
        sentiment: 0.6,
        importance: 0.2,
    },
    TestCase {
        title: "A new Register Allocator for ZJIT",
        summary: "We recently landed a new register allocator in ZJIT. Learn about lifetimes, interference graphs, and linear scan - and why a global allocator unlocks features like method inlining.",
        category: Category::Software,
        region: Region::Global,
        sentiment: 0.6,
        importance: 0.1,
    },
    TestCase {
        title: "Li Jiaying: China launches first Hong Kong astronaut into space",
        summary: "The 43-year-old police officer and mother of three serves as the team's payload scientist",
        category: Category::Science,
        region: Region::China,
        sentiment: 0.9,
        importance: 0.3,
    },
    TestCase {
        title: "I used the ChatGPT ‘circus’ prompt — and it's a surprisingly effective prioritization hack for multitasking",
        summary: "When I'm juggling a lot, this prompt is my secret to seeing my day more clearly",
        category: Category::AI,
        region: Region::Global,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "French Open 2026 results: Tamara Korpatsch and Wang Xinyu have heated exchange and avoid handshake in Roland Garros row",
        summary: "Germany's Tamara Korpatsch says it would be embarrassing if she cheated against China's Wang Xinyu following their heated exchange at the French Open.",
        category: Category::Sports,
        region: Region::France,
        sentiment: 0.3,
        importance: 0.2,
    },
    TestCase {
        title: "Sony is offering up to 50 percent off some of our favorite PS5 games",
        summary: "Sony is discounting a host of PS5 games as part of its latest Days of Play promo. You can also save on Govee's Lightwall and the Switch port of Red Dead Redemption.",
        category: Category::Gaming,
        region: Region::Global,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "The £5 coffee that tells a story of global economic turmoil",
        summary: "Coffees at some city centre outlets now cost £5. It's a story of tariffs, the climate, Gen Z cultural tastes, and savvy coffee farmers playing the market.",
        category: Category::Business,
        region: Region::UnitedKingdom,
        sentiment: 0.2,
        importance: 0.6,
    },
    TestCase {
        title: "Most Britons think water and energy companies should be nationalised",
        summary: "Support for nationalisation of various sectors has remained constant since the last election.",
        category: Category::Politics,
        region: Region::UnitedKingdom,
        sentiment: 0.5,
        importance: 0.7,
    },
    TestCase {
        title: "Students react to ratification of Restore Britain society by University of York",
        summary: "Your right to be here is not a pub-crawl punchline.",
        category: Category::Politics,
        region: Region::England,
        sentiment: 0.5,
        importance: 0.4,
    },
    TestCase {
        title: "Israel begins strikes on southern Lebanon after evacuation orders",
        summary: "The military says areas south of the Zahrani River are now combat zones as it begins fresh strikes.",
        category: Category::Politics,
        region: Region::MiddleEastNorthAfrica,
        sentiment: 0.1,
        importance: 0.5,
    },
    TestCase {
        title: "Calls for Barnsley Reform councillor to resign over apparent swastika tattoo",
        summary: "Publicly accessible Facebook images dating back to 2018 appear to show the symbol on Andy Arnold's arm.",
        category: Category::Politics,
        region: Region::England,
        sentiment: 0.2,
        importance: 0.4,
    },
    TestCase {
        title: "A Google employee allegedly used inside information to win $1.2 million on Polymarket",
        summary: "Federal prosecutors have charged a Google employee with fraud after he allegedly used confidential data to make over $1 million on bets.",
        category: Category::Law,
        region: Region::UnitedStates,
        sentiment: 0.3,
        importance: 0.4,
    },
    TestCase {
        title: "It's not a nice world out there: Birth rates hit a 50-year low",
        summary: "Live births in England and Wales are at their lowest since 1977, while the age of first-time mothers has also risen.",
        category: Category::Health,
        region: Region::UnitedKingdom,
        sentiment: 0.2,
        importance: 0.7,
    },
    TestCase {
        title: "Britain's rudest chalk figure gets a glow-up to stop it fading in the rain",
        summary: "National Trust staff and volunteers will apply 17 tonnes of fresh chalk to the outline of the famous figure.",
        category: Category::Culture,
        region: Region::England,
        sentiment: 0.8,
        importance: 0.3,
    },
    TestCase {
        title: "Upgrade your grilling — 12 essential buys reduced at Amazon right now",
        summary: "The hottest grilling accessories are discounted, with savings of up to 50%.",
        category: Category::Lifestyle,
        region: Region::UnitedStates,
        sentiment: 0.7,
        importance: 0.0,
    },
    TestCase {
        title: "I found 25 Amazon summer essentials under $50 — beach gear, patio upgrades, gardening tools and more",
        summary: "Gear up for summer with these affordable deals.",
        category: Category::Lifestyle,
        region: Region::UnitedStates,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Currys takes up to 48% off fans, air conditioners, BBQs and ice cream makers",
        summary: "Keep cool with up to 48% off essential summer appliances.",
        category: Category::Lifestyle,
        region: Region::Global,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Britain's protected birds of prey still illegally targeted",
        summary: "The charity says more than half of attacks happened on or near land managed for game shooting.",
        category: Category::Nature,
        region: Region::UnitedKingdom,
        sentiment: 0.1,
        importance: 0.7,
    },
    TestCase {
        title: "'Bone idle' visitors slammed as beach left covered in nappies and chicken",
        summary: "Nicola Jameson described the scene as horrendous.",
        category: Category::Nature,
        region: Region::Global,
        sentiment: 0.2,
        importance: 0.3,
    },
    TestCase {
        title: "National Trust appeal after Cardingmill Valley littering",
        summary: "Litter was left all over Cardingmill Valley after an exceptionally busy bank holiday weekend.",
        category: Category::Nature,
        region: Region::England,
        sentiment: 0.3,
        importance: 0.4,
    },
    TestCase {
        title: "Best Buy's World Cup sale is on — here are the 5 trophy-winning TVs you should check out",
        summary: "Save big on a new screen ahead of kickoff.",
        category: Category::Technology,
        region: Region::UnitedStates,
        sentiment: 0.7,
        importance: 0.0,
    },
    TestCase {
        title: "Apple's newest iPad Air is up to $100 off for the first time",
        summary: "Apple's M4-powered iPad Air is on sale starting at $519.99. The tablet offers faster performance than its predecessor.",
        category: Category::Technology,
        region: Region::UnitedStates,
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "First UK team to use Google Willow quantum chip announced",
        summary: "King's College London researchers hope the chip will help answer previously unanswerable questions about the most important natural processes.",
        category: Category::Science,
        region: Region::UnitedKingdom,
        sentiment: 0.9,
        importance: 0.7,
    },
];

#[test]
fn category_classification() -> Result<()> {
    let mut failures = Vec::new();

    for (case, scores) in TEST_CASES.iter().zip(label_scores("category")?) {
        let category = best_label::<Category>(&scores, "category")?;
        if category != case.category {
            failures.push(format!(
                "{category:<10} expected {:<10} Title: {}. Summary: {}",
                case.category, case.title, case.summary
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "\n{} category cases misclassified:\n\n{}",
        failures.len(),
        failures.join("\n")
    );

    Ok(())
}

#[test]
fn region_classification() -> Result<()> {
    let mut failures = Vec::new();

    for (case, scores) in TEST_CASES.iter().zip(label_scores("region")?) {
        let region = best_label::<Region>(&scores, "region")?;

        // Define acceptable geographic overlaps
        let is_acceptable_match = match (&region, &case.region) {
            // Perfect match is always fine
            (a, b) if a == b => true,
            // Cross-compatibility between UK and its home nations
            (Region::UnitedKingdom, Region::England | Region::Scotland | Region::Wales)
            | (Region::England | Region::Scotland | Region::Wales, Region::UnitedKingdom) => true,
            // Any other combo (e.g., classifying England as Scotland, or UK as Global) is a failure
            _ => false,
        };

        if !is_acceptable_match {
            failures.push(format!(
                "{region:<10} expected {:<10} Title: {}. Summary: {}",
                case.region, case.title, case.summary
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "\n{} region cases misclassified:\n\n{}",
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
        let expected_min = expected - PLUS_MINUS_MARGIN;
        let expected_max = expected + PLUS_MINUS_MARGIN;

        if !(expected_min..=expected_max).contains(&score) {
            failures.push(format!(
                "{score:>5.3}  {:.2}..={:.2}  Title: {}. Summary: {}",
                expected_min.clamp(0.0, 1.0),
                expected_max.clamp(0.0, 1.0),
                case.title,
                case.summary,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "\n{} {label_group} cases outside expected range (±{PLUS_MINUS_MARGIN}):\n\n{}",
        failures.len(),
        failures.join("\n")
    );

    Ok(())
}
