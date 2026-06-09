use crate::{
    server::{
        database::{self, LabelScore},
        embeddings::{
            EMBEDDING_TITLE_REPEAT, article_text, best_label, binary_label_score,
            generate_embeddings, maintenance_label_embeddings,
        },
    },
    shared::{Category, PendingSource, Region},
};
use anyhow::{Context, Result};
use itertools::Itertools;
use std::sync::OnceLock;

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

const SCORE_EXPECTED_MAX_DISTANCE: f32 = 0.05;

struct TestCase {
    title: &'static str,
    summary: &'static str,
    category: Category,
    regions: &'static [Region],
    sentiment: f32,
    importance: f32,
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        title: "Aldi announces £5bn investment in UK food production",
        summary: "Aldi has pledged over £5 billion in long-term supplier agreements, in a move which will enable its suppliers to invest in future operations.",
        category: Category::Business,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.7,
        importance: 0.4,
    },
    TestCase {
        title: "Manchester University to offer work placements to all undergraduates",
        summary: "Russell Group university promises students, from chemical engineering to classics, ‘meaningful real-world experience’",
        category: Category::Business,
        regions: &[Region::England, Region::UnitedKingdom],
        sentiment: 0.8,
        importance: 0.6,
    },
    TestCase {
        title: "Scottish first-time buyers offered up to £10,000 interest-free loans",
        summary: "The Scottish government has launched the First Homes Fund for those looking to get on the property ladder.",
        category: Category::Politics,
        regions: &[Region::Scotland],
        sentiment: 0.8,
        importance: 0.7,
    },
    TestCase {
        title: "Reform UK civil service plan 'would sack more planning officers than exist'",
        summary: "Analysis of party's proposed cuts also suggests it would get rid of two-thirds of psychologists who support prison staff.",
        category: Category::Politics,
        regions: &[Region::UnitedKingdom, Region::England],
        sentiment: 0.3,
        importance: 0.5,
    },
    TestCase {
        title: "Jill Biden says she thought Joe Biden was having a stroke during 2024 debate",
        summary: "The former US first lady told CBS News that she was frightened by Joe Biden's performance against Donald Trump.",
        category: Category::Politics,
        regions: &[Region::UnitedStates],
        sentiment: 0.3,
        importance: 0.6,
    },
    TestCase {
        title: "Russia is targeting UK’s infrastructure and democracy, GCHQ head to say",
        summary: "Anne Keast-Butler will also warn of narrowing window to stay ahead of China in ‘new era of radical uncertainty’",
        category: Category::Politics,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.2,
        importance: 0.8,
    },
    TestCase {
        title: "Flowers laid as teenage girl found dead in water at Kingsbury country park",
        summary: "The girl's body has been recovered from the water following concerns for her welfare, police say.",
        category: Category::Law,
        regions: &[Region::England],
        sentiment: 0.2,
        importance: 0.3,
    },
    TestCase {
        title: "Woman warned to prepare for jail after XL Bully kills Shrewsbury teenager",
        summary: "Teenager Morgan Dorsett, 19, suffered fatal bites to her neck in the XL Bully attack.",
        category: Category::Law,
        regions: &[Region::England],
        sentiment: 0.1,
        importance: 0.3,
    },
    TestCase {
        title: "Australia sues 3M over 'forever chemicals' in firefighting foam",
        summary: "The A$2bn case, which centres on contamination at defence sites, is the largest ever brought by the government.",
        category: Category::Law,
        regions: &[Region::Oceania],
        sentiment: 0.3,
        importance: 0.5,
    },
    TestCase {
        title: "Men arrested after alleged sexual assault of teens on beach.",
        summary: "Two men were arrested on Bournemouth Beach yesterday on suspicion of sexually assaulting two teenage girls.",
        category: Category::Law,
        regions: &[Region::England],
        sentiment: 0.2,
        importance: 0.4,
    },
    TestCase {
        title: "Utumishi Girls school fire: Sixteen students killed in Kenya school blaze.",
        summary: "Search-and-rescue operations are ongoing, with the number of casualties yet to be confirmed",
        category: Category::Law,
        regions: &[Region::SubSaharanAfrica],
        sentiment: 0.0,
        importance: 0.7,
    },
    TestCase {
        title: "Teenage patients 'ignored' before fatal NHS trust failures",
        summary: "Mental health patients say nobody listened to their concerns about a north-east England trust.",
        category: Category::Health,
        regions: &[Region::England],
        sentiment: 0.0,
        importance: 0.5,
    },
    TestCase {
        title: "Former US Attorney General Pam Bondi diagnosed with cancer",
        summary: "Bondi, who was diagnosed shortly after being ousted in April, is also joining the White House's new advisory council on AI.",
        category: Category::Health,
        regions: &[Region::UnitedStates],
        sentiment: 0.1,
        importance: 0.5,
    },
    TestCase {
        title: "How to watch 'A Good Girl's Guide To Murder' season 2 online - stream the hit crime-thriller series for FREE",
        summary: "Pip and Ravi are back for more mystery-solving with season 2 of A Good Girl's Guide To Murder and you can watch along for free.",
        category: Category::Culture,
        regions: &[Region::Global],
        sentiment: 0.6,
        importance: 0.0,
    },
    TestCase {
        title: "Energy bills to rise for millions as impact of Iran war hits",
        summary: "A household using a typical amount of energy will pay £221 a year more, under the regulator's new price cap.",
        category: Category::Lifestyle,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.0,
        importance: 0.8,
    },
    TestCase {
        title: "The Saatva mattress I recommend to everyone is $400 off in a huge last-chance Memorial Day sale",
        summary: "The Saatva Classic is our number one rated mattress.",
        category: Category::Lifestyle,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Skechers walking and running shoes are still discounted after Memorial Day — 13 best deals with up to 48% off",
        summary: "Skechers walking and running shoes",
        category: Category::Lifestyle,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "All the news about Ferrari’s polarizing Luce EV",
        summary: "The first electric car from Ferrari is here, and the design of the Luce has fans saying it doesn’t look like a true Ferrari.",
        category: Category::Transport,
        regions: &[Region::Global],
        sentiment: 0.7,
        importance: 0.4,
    },
    TestCase {
        title: "Why temperature records are being not only broken but smashed",
        summary: "The combined effects of a heat dome and climate change have brought extreme warmth to western Europe.",
        category: Category::Nature,
        regions: &[Region::WesternEurope],
        sentiment: 0.2,
        importance: 0.8,
    },
    TestCase {
        title: "Tom's Guide to tomatoes",
        summary: "Everything you need to know about planting, growing, pruning, and harvesting tomatoes.",
        category: Category::Nature,
        regions: &[Region::Global],
        sentiment: 0.6,
        importance: 0.2,
    },
    TestCase {
        title: "A rare ancient rainforest set to come back to life",
        summary: "Ulster Wildlife takes on 100 year restoration project of ancient rainforest.",
        category: Category::Nature,
        regions: &[Region::Ireland],
        sentiment: 0.8,
        importance: 0.5,
    },
    TestCase {
        title: "River Waveney land purchased to save it from risk of development",
        summary: "A council hopes to enhance wildlife in the area and improve public access.",
        category: Category::Nature,
        regions: &[Region::England],
        sentiment: 0.8,
        importance: 0.6,
    },
    TestCase {
        title: "Dead sharks and fish found washed up on Welsh beaches",
        summary: "The dead fish, at Saundersfoot and Pembrey beaches, were probably part of a discarded catch.",
        category: Category::Nature,
        regions: &[Region::Wales],
        sentiment: 0.2,
        importance: 0.4,
    },
    TestCase {
        title: "Peebles river detectives part of network tackling pollution",
        summary: "Hundreds of volunteers survey Scotland's watercourses and report any issues of concern quickly",
        category: Category::Nature,
        regions: &[Region::Scotland],
        sentiment: 0.8,
        importance: 0.5,
    },
    TestCase {
        title: "I put aluminum foil behind my router to fix my Wi-Fi dead zones — and it actually worked",
        summary: "Curved aluminum foil behind your router reflects Wi-Fi signals directly toward your dead zones, instead of letting them scatter",
        category: Category::Technology,
        regions: &[Region::Global],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Oura Ring 5 could be unveiled this week with new design — here's why I'm concerned. Smart Rings.",
        summary: "A thinner design could have implications for battery life",
        category: Category::Technology,
        regions: &[Region::Global],
        sentiment: 0.4,
        importance: 0.1,
    },
    TestCase {
        title: "The new Halide camera app launches with film looks and an upgraded photo editor.",
        summary: "Halide Mark III is now available for iPhones and iPads with an updated interface, new film looks, and an upgraded editor that can process third-party RAW files.",
        category: Category::Software,
        regions: &[Region::Global],
        sentiment: 0.7,
        importance: 0.2,
    },
    TestCase {
        title: "space-tree: Workspace Management Trees in Emacs",
        summary: "space-tree is a tree-based workspace manager for Emacs. Workspaces are a battle-tested UX concept across operating systems, but in Emacs and most OSes alike.",
        category: Category::Software,
        regions: &[Region::Global],
        sentiment: 0.6,
        importance: 0.2,
    },
    TestCase {
        title: "A new Register Allocator for ZJIT",
        summary: "We recently landed a new register allocator in ZJIT. Learn about lifetimes, interference graphs, and linear scan - and why a global allocator unlocks features like method inlining.",
        category: Category::Software,
        regions: &[Region::Global],
        sentiment: 0.6,
        importance: 0.1,
    },
    TestCase {
        title: "Li Jiaying: China launches first Hong Kong astronaut into space",
        summary: "The 43-year-old police officer and mother of three serves as the team's payload scientist",
        category: Category::Science,
        regions: &[Region::China],
        sentiment: 0.9,
        importance: 0.3,
    },
    TestCase {
        title: "I used the ChatGPT ‘circus’ prompt — and it's a surprisingly effective prioritization hack for multitasking",
        summary: "When I'm juggling a lot, this prompt is my secret to seeing my day more clearly",
        category: Category::AI,
        regions: &[Region::Global],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "French Open 2026 results: Tamara Korpatsch and Wang Xinyu have heated exchange and avoid handshake in Roland Garros row",
        summary: "Germany's Tamara Korpatsch says it would be embarrassing if she cheated against China's Wang Xinyu following their heated exchange at the French Open.",
        category: Category::Sports,
        regions: &[Region::France],
        sentiment: 0.3,
        importance: 0.2,
    },
    TestCase {
        title: "Sony is offering up to 50 percent off some of our favorite PS5 games",
        summary: "Sony is discounting a host of PS5 games as part of its latest Days of Play promo. You can also save on Govee's Lightwall and the Switch port of Red Dead Redemption.",
        category: Category::Gaming,
        regions: &[Region::Global],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "The £5 coffee that tells a story of global economic turmoil",
        summary: "Coffees at some city centre outlets now cost £5. It's a story of tariffs, the climate, Gen Z cultural tastes, and savvy coffee farmers playing the market.",
        category: Category::Business,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.2,
        importance: 0.6,
    },
    TestCase {
        title: "Most Britons think water and energy companies should be nationalised",
        summary: "Support for nationalisation of various sectors has remained constant since the last election.",
        category: Category::Politics,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.5,
        importance: 0.7,
    },
    TestCase {
        title: "Students react to ratification of Restore Britain society by University of York",
        summary: "Your right to be here is not a pub-crawl punchline.",
        category: Category::Politics,
        regions: &[Region::England, Region::UnitedKingdom],
        sentiment: 0.5,
        importance: 0.4,
    },
    TestCase {
        title: "Israel begins strikes on southern Lebanon after evacuation orders",
        summary: "The military says areas south of the Zahrani River are now combat zones as it begins fresh strikes.",
        category: Category::Politics,
        regions: &[Region::MiddleEastNorthAfrica],
        sentiment: 0.1,
        importance: 0.5,
    },
    TestCase {
        title: "Calls for Barnsley Reform councillor to resign over apparent swastika tattoo",
        summary: "Publicly accessible Facebook images dating back to 2018 appear to show the symbol on Andy Arnold's arm.",
        category: Category::Politics,
        regions: &[Region::England],
        sentiment: 0.2,
        importance: 0.4,
    },
    TestCase {
        title: "A Google employee allegedly used inside information to win $1.2 million on Polymarket",
        summary: "Federal prosecutors have charged a Google employee with fraud after he allegedly used confidential data to make over $1 million on bets.",
        category: Category::Law,
        regions: &[Region::UnitedStates],
        sentiment: 0.3,
        importance: 0.4,
    },
    TestCase {
        title: "It's not a nice world out there: Birth rates hit a 50-year low",
        summary: "Live births in England and Wales are at their lowest since 1977, while the age of first-time mothers has also risen.",
        category: Category::Health,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.2,
        importance: 0.7,
    },
    TestCase {
        title: "Britain's rudest chalk figure gets a glow-up to stop it fading in the rain",
        summary: "National Trust staff and volunteers will apply 17 tonnes of fresh chalk to the outline of the famous figure.",
        category: Category::Culture,
        regions: &[Region::England, Region::UnitedKingdom],
        sentiment: 0.8,
        importance: 0.3,
    },
    TestCase {
        title: "Upgrade your grilling — 12 essential buys reduced at Amazon right now",
        summary: "The hottest grilling accessories are discounted, with savings of up to 50%.",
        category: Category::Lifestyle,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.7,
        importance: 0.0,
    },
    TestCase {
        title: "I found 25 Amazon summer essentials under $50 — beach gear, patio upgrades, gardening tools and more",
        summary: "Gear up for summer with these affordable deals.",
        category: Category::Lifestyle,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Currys takes up to 48% off fans, air conditioners, BBQs and ice cream makers",
        summary: "Keep cool with up to 48% off essential summer appliances.",
        category: Category::Lifestyle,
        regions: &[Region::UnitedKingdom, Region::Global],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Britain's protected birds of prey still illegally targeted",
        summary: "The charity says more than half of attacks happened on or near land managed for game shooting.",
        category: Category::Nature,
        regions: &[Region::UnitedKingdom, Region::England],
        sentiment: 0.1,
        importance: 0.7,
    },
    TestCase {
        title: "'Bone idle' visitors slammed as beach left covered in nappies and chicken",
        summary: "Nicola Jameson described the scene as horrendous.",
        category: Category::Nature,
        regions: &[Region::Global, Region::UnitedKingdom, Region::England],
        sentiment: 0.2,
        importance: 0.3,
    },
    TestCase {
        title: "National Trust appeal after Cardingmill Valley littering",
        summary: "Litter was left all over Cardingmill Valley after an exceptionally busy bank holiday weekend.",
        category: Category::Nature,
        regions: &[Region::England, Region::UnitedKingdom],
        sentiment: 0.3,
        importance: 0.4,
    },
    TestCase {
        title: "Best Buy's World Cup sale is on — here are the 5 trophy-winning TVs you should check out",
        summary: "Save big on a new screen ahead of kickoff.",
        category: Category::Technology,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.7,
        importance: 0.0,
    },
    TestCase {
        title: "Apple's newest iPad Air is up to $100 off for the first time",
        summary: "Apple's M4-powered iPad Air is on sale starting at $519.99. The tablet offers faster performance than its predecessor.",
        category: Category::Technology,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "First UK team to use Google Willow quantum chip announced",
        summary: "King's College London researchers hope the chip will help answer previously unanswerable questions about the most important natural processes.",
        category: Category::Science,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.9,
        importance: 0.7,
    },
    TestCase {
        title: "eufyMake E1 UV Printer drops by £350 in limited early bird flash sale",
        summary: "The innovative 3D UV resin printer gets a massive price cut ahead of its official retail debut, including bundling £500 worth of free printing accessories.",
        category: Category::Technology,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.7,
        importance: 0.2,
    },
    TestCase {
        title: "Save a whopping £350 on the eufyMake E1 UV Printer with early access deals before May 31st.",
        summary: "Save a whopping £350 on the eufyMake E1 UV Printer + £500 in free gifts during the early access sale ending May 31st",
        category: Category::Technology,
        regions: &[Region::UnitedKingdom, Region::Global],
        sentiment: 0.6,
        importance: 0.1,
    },
    TestCase {
        title: "Here’s where you can preorder the new Oura Ring 5.",
        summary: "The Ring 5 is smaller than its predecessor and comes with new health, fitness, and AI-powered tools. It’s also already available for preorder starting at $399.",
        category: Category::Technology,
        regions: &[Region::UnitedStates, Region::Global],
        sentiment: 0.6,
        importance: 0.1,
    },
    TestCase {
        title: "What’s next for Microsoft’s Surface PCs?.",
        summary: "The future of Surface looks like a much smaller lineup of devices.",
        category: Category::Technology,
        regions: &[Region::Global],
        sentiment: 0.8,
        importance: 0.3,
    },
    TestCase {
        title: "Oil prices fall after report of breakthrough in US-Iran talks.",
        summary: "A report of a extended ceasefire, subject to Donald Trump's approval, has led to global oil prices to fall on Thursday.",
        category: Category::Business,
        regions: &[Region::Global, Region::MiddleEastNorthAfrica],
        sentiment: 0.6,
        importance: 0.9,
    },
    TestCase {
        title: "New dark skies policy could 'help Highland night-time economy'.",
        summary: "Highland Council says it already has some of the darkest skies in Europe for stargazers to enjoy.",
        category: Category::Nature,
        regions: &[Region::Scotland],
        sentiment: 0.8,
        importance: 0.3,
    },
    TestCase {
        title: "Talbot Green murder of Joanne Penney planned from a prison cell.",
        summary: "Renaldo Baptiste used a hidden \"prison phone\" to supply the gun that killed Joanne Penney in 2025.",
        category: Category::Law,
        regions: &[Region::UnitedKingdom, Region::Wales],
        sentiment: 0.1,
        importance: 0.4,
    },
    TestCase {
        title: "Mum who murdered 19-day-old daughter in 'brutal attack' tried to blame it on a child.",
        summary: "Nicole Blain was filmed storming off in a huff at her gender reveal party after discovering she was expecting a girl. The sick mum has been jailed for at least 19 years",
        category: Category::Law,
        regions: &[Region::UnitedKingdom, Region::Scotland],
        sentiment: 0.1,
        importance: 0.4,
    },
    TestCase {
        title: "Isle of Man TT competitor killed in qualifying crash.",
        summary: "Daniel Ingham, 33, died in the crash during qualifying for the Isle of Man TT on Wednesday.",
        category: Category::Transport,
        regions: &[Region::UnitedKingdom, Region::England],
        sentiment: 0.1,
        importance: 0.3,
    },
    TestCase {
        title: "Teenage boys die in Kent and Oxford in latest heatwave water deaths.",
        summary: "The body of a boy is recovered from a pond in Kent, while another teen dies in the River Thames in Oxford.",
        category: Category::Nature,
        regions: &[Region::England, Region::UnitedKingdom],
        sentiment: 0.0,
        importance: 0.5,
    },
    TestCase {
        title: "I just want freedom from fear, says girl raped by teenage boys as their sentences to be reviewed.",
        summary: "The teenager tells BBC Newsnight she is suffering from flashbacks and living in fear following the attack.",
        category: Category::Law,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.0,
        importance: 0.5,
    },
    TestCase {
        title: "Gaza City hospitals say several killed in strike, as Israel targets Hamas leaders",
        summary: "At least five children were reportedly killed in the strike, which appears to have targeted a Hamas commander.",
        category: Category::Politics,
        regions: &[Region::MiddleEastNorthAfrica],
        sentiment: 0.0,
        importance: 0.8,
    },
    TestCase {
        title: "3 best new to Netflix shows you should binge-watch this weekend",
        summary: "Add these new releases to your Netflix watchlist for a weekend streaming marathon.",
        category: Category::Culture,
        regions: &[Region::Global],
        sentiment: 0.6,
        importance: 0.0,
    },
    TestCase {
        title: "Don't wait until Prime Day! These Amazon devices are already on sale",
        summary: "Limited time deals across tech, security and entertainment.",
        category: Category::Technology,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.6,
        importance: 0.0,
    },
    TestCase {
        title: "Epic sneaker sale knocks up to 50% off Saucony, Brooks, and Asics ahead of Prime Day",
        summary: "Upgrade your mileage with these sneaker deals.",
        category: Category::Lifestyle,
        regions: &[Region::Global, Region::UnitedStates],
        sentiment: 0.6,
        importance: 0.0,
    },
    TestCase {
        title: "How to watch Canada vs Republic of Ireland on RTE Player",
        summary: "Instructions for streaming the match for free online.",
        category: Category::Sports,
        regions: &[Region::Ireland, Region::Canada],
        sentiment: 0.5,
        importance: 0.0,
    },
    TestCase {
        title: "Everest guide survived six-day ordeal by eating chocolate and chewing ice",
        summary: "Dawa Sherpa was spotted alive by a cleaning crew as he slid slowly down the world's tallest mountain and spoke from hospital.",
        category: Category::Lifestyle,
        regions: &[Region::IndianSubcontinent],
        sentiment: 0.5,
        importance: 0.3,
    },
    TestCase {
        title: "Mum survived a cancer misdiagnosis but dad then took his own life",
        summary: "Mary Crowley was wrongly diagnosed with terminal cancer and husband David could not cope with their ordeal.",
        category: Category::Health,
        regions: &[Region::UnitedKingdom, Region::Global],
        sentiment: 0.0,
        importance: 0.4,
    },
    TestCase {
        title: "Teen rapists spared jail partly because of intellectual limitations, judge's remarks show",
        summary: "The boys' sentencing caused an outcry when they were given youth rehabilitation orders.",
        category: Category::Law,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.0,
        importance: 0.5,
    },
    TestCase {
        title: "Former student in court over University of Surrey crossbow attack",
        summary: "A university campus safety officer was seriously injured in the attack in Guildford.",
        category: Category::Law,
        regions: &[Region::England],
        sentiment: 0.1,
        importance: 0.5,
    },
    TestCase {
        title: "Putin says there is no point meeting Zelensky over ending Ukraine war",
        summary: "The Russian president's refusal comes after his Ukrainian counterpart called for face-to-face talks.",
        category: Category::Politics,
        regions: &[Region::EasternEurope],
        sentiment: 0.1,
        importance: 0.7,
    },
    TestCase {
        title: "US stocks slump as fears over Big Tech shake Wall Street",
        summary: "The Nasdaq saw its biggest daily fall since early 2025.",
        category: Category::Business,
        regions: &[Region::UnitedStates],
        sentiment: 0.0,
        importance: 0.8,
    },
    TestCase {
        title: "Lloyds, Halifax and Bank of Scotland app users report outage",
        summary: "The bank said some customers were having issues with app and online banking access.",
        category: Category::Software,
        regions: &[Region::UnitedKingdom],
        sentiment: 0.1,
        importance: 0.4,
    },
    TestCase {
        title: "New York lawmakers pass one-year ban on new data centers",
        summary: "The state legislature passed a ban on new large data centers to assess environmental impact.",
        category: Category::Politics,
        regions: &[Region::UnitedStates],
        sentiment: 0.5,
        importance: 0.6,
    },
    TestCase {
        title: "Jared Kushner-backed luxury resort plan sparks protest in Albania",
        summary: "Demonstrators say the project would harm a protected environment.",
        category: Category::Nature,
        regions: &[Region::Balkans],
        sentiment: 0.2,
        importance: 0.3,
    },
    TestCase {
        title: "Monaco Grand Prix: Lewis Hamilton heads Charles Leclerc as Ferrari dominate practice",
        summary: "Hamilton set the pace as Ferrari dominated Friday practice at the Monaco Grand Prix.",
        category: Category::Sports,
        regions: &[Region::WesternEurope],
        sentiment: 0.8,
        importance: 0.2,
    },
    TestCase {
        title: "We’re hosting a Coding Agent Benchmarks event with lightning talks and a panel",
        summary: "The event will gather AI researchers, builders, and engineers in San Francisco.",
        category: Category::AI,
        regions: &[Region::UnitedStates],
        sentiment: 0.7,
        importance: 0.1,
    },
    TestCase {
        title: "Paralympian John McFall could be first astronaut with disability in orbit",
        summary: "John McFall has moved closer to becoming the first disabled astronaut in orbit.",
        category: Category::Science,
        regions: &[Region::Global, Region::UnitedKingdom],
        sentiment: 0.8,
        importance: 0.5,
    },
    TestCase {
        title: "One in four births in England are now emergency caesareans, analysis shows",
        summary: "The shift marks a significant rise over five years, but experts say there is no single clear explanation.",
        category: Category::Health,
        regions: &[Region::UnitedKingdom, Region::England],
        sentiment: 0.2,
        importance: 0.7,
    },
    TestCase {
        title: "Nests and habitats destroyed as Farlington flood defences fail",
        summary: "An internationally important nature reserve in Hampshire is under threat from failing flood defences.",
        category: Category::Nature,
        regions: &[Region::England, Region::UnitedKingdom],
        sentiment: 0.0,
        importance: 0.7,
    },
    TestCase {
        title: "Oil prices edge higher after strikes on Israel test ceasefire",
        summary: "Iran said the attacks, its first since an April ceasefire, are the start of 'a full week' of strikes",
        category: Category::Politics,
        regions: &[Region::MiddleEastNorthAfrica, Region::Global],
        sentiment: 0.0,
        importance: 1.0,
    },
    TestCase {
        title: "My drinking days are over. Here are my tips for an alcohol-free World Cup",
        summary: "Sober or sober-curious? One football fan shares his tips for having a great alcohol-free World Cup.",
        category: Category::Sports,
        regions: &[Region::Global],
        sentiment: 0.6,
        importance: 0.0,
    },
    TestCase {
        title: "Curlews: Endangered bird eggs hatch after being saved from wildfire",
        summary: "The four chicks, which hatched from the eggs, are being cared for until they are old and strong enough to be released.",
        category: Category::Nature,
        regions: &[Region::Global],
        sentiment: 0.8,
        importance: 0.3,
    },
    TestCase {
        title: "Top five-a-day foods new study says your heart needs",
        summary: "Not all fruit and veg is equal for getting nutrients called flavanols, say researchers.",
        category: Category::Health,
        regions: &[Region::Global],
        sentiment: 0.7,
        importance: 0.2,
    },
    TestCase {
        title: "New drug to stop 'Ozempic butt' muscle loss side effect of obesity jabs",
        summary: "A third of the weight loss from obesity jabs can come from muscle, say experts.",
        category: Category::Health,
        regions: &[Region::Global],
        sentiment: 0.8,
        importance: 0.5,
    },
    TestCase {
        title: "I use AI every morning before 9 a.m. — these 5 habits make my day easier",
        summary: "From coffee to conference calls, these simple AI routines help me stay organized, calmer and more productive",
        category: Category::AI,
        regions: &[Region::Global],
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
        if !case.regions.contains(&region) {
            let expected = case
                .regions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("|");
            failures.push(format!(
                "{region:<10} expected {expected:<10} Title: {}. Summary: {}",
                case.title, case.summary
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

static LABEL_EMBEDDINGS_READY: OnceLock<Result<(), String>> = OnceLock::new();

fn label_scores(group: &str) -> Result<Vec<Vec<LabelScore>>> {
    let texts = TEST_CASES
        .iter()
        .map(|case| {
            article_text(
                &PendingSource {
                    title: case.title.to_string(),
                    summary: case.summary.to_string(),
                    ..Default::default()
                },
                EMBEDDING_TITLE_REPEAT,
            )
        })
        .collect::<Vec<_>>();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create Tokio runtime")?;

    LABEL_EMBEDDINGS_READY
        .get_or_init(|| {
            rt.block_on(async {
                if let Err(err) = database::init().await {
                    let message = err.to_string();
                    if !message.contains("Database already initialised") {
                        return Err(message);
                    }
                }

                maintenance_label_embeddings()
                    .await
                    .map_err(|err| err.to_string())
            })
        })
        .clone()
        .map_err(anyhow::Error::msg)?;

    let embeddings = rt.block_on(generate_embeddings(&texts))?;

    embeddings
        .iter()
        .map(|embedding| {
            Ok(rt
                .block_on(database::get_label_scores(embedding))?
                .into_iter()
                .filter(|score| score.label_group == group)
                .collect())
        })
        .collect()
}

fn assert_binary_scores(
    label_group: &str,
    positive_value: &str,
    expected_score: impl Fn(&TestCase) -> f32,
) -> Result<()> {
    let mut total_distance = 0.0;
    let mut score_distances = Vec::new();

    for (case, scores) in TEST_CASES.iter().zip(label_scores(label_group)?) {
        let score = binary_label_score(&scores, label_group, positive_value)?;
        let expected = expected_score(case);
        let distance = (score - expected).abs();
        total_distance += distance;
        let score_detail = binary_score_detail(&scores, positive_value);
        score_distances.push((
            distance,
            score,
            expected,
            case.title,
            case.summary,
            score_detail,
        ));
    }

    // Sort score_distances by distance
    score_distances.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    total_distance /= score_distances.len() as f32;

    println!("{label_group} total target distance: {total_distance:.3}");

    assert!(
        total_distance < SCORE_EXPECTED_MAX_DISTANCE,
        "\nMost distant cases:\n\n{}",
        score_distances
            .iter()
            .take(10)
            .map(
                |(distance, score, expected, title, summary, score_detail)| {
                    let distance = format!(
                        "{}{:.2}",
                        if score > expected { "↑" } else { "↓" },
                        distance
                    );
                    format!("{distance}  score:{score:.2} expected:{expected:.2} Title: {title}. Summary: {summary}\n{score_detail}")
                }
            )
            .join("\n\n"),
    );

    Ok(())
}

fn binary_score_detail(scores: &[LabelScore], positive_value: &str) -> String {
    let mut positive = Vec::new();
    let mut negative = Vec::new();

    for score in scores {
        if score.label_value == positive_value {
            positive.push(score);
        } else {
            negative.push(score);
        }
    }

    format!(
        "    top positive: {}\n    top negative: {}",
        label_list(&positive),
        label_list(&negative),
    )
}

fn label_list(scores: &[&LabelScore]) -> String {
    let mut sorted = scores.to_vec();
    sorted.sort_by(|left, right| right.similarity.total_cmp(&left.similarity));

    format!(
        "[{}]",
        sorted
            .iter()
            .take(5)
            .map(|score| format!("{:.3} {}", score.similarity, label_snippet(&score.text)))
            .collect::<Vec<_>>()
            .join(" | ")
    )
}

fn label_snippet(text: &str) -> String {
    const MAX_CHARS: usize = 60;
    let mut snippet = text.chars().take(MAX_CHARS).collect::<String>();
    if text.chars().count() > MAX_CHARS {
        snippet.push_str("...");
    }
    snippet
}
