use crate::shared::{ArticleSource, ArticleType, Category};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::LazyLock;
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, OnceCell};

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const MODEL_NAME: &str = "EmbeddingGemma300M";

/// Positive and negative labels for each category
const fn category_labels(category: Category) -> (&'static [&'static str], &'static [&'static str]) {
    match category {
        Category::Business => (
            &[
                "Business. Corporate finance, economic trends, startup growth, market investments, earnings reports.",
                "Apple reports record quarterly earnings as iPhone sales surge to $95bn. The tech giant beat analyst expectations driven by strong services growth and emerging-market demand.",
                "Goldman Sachs to cut 3,000 jobs as investment banking revenue slumps. The Wall Street giant is restructuring its consumer division after a costly retreat from retail banking.",
                "Kickstarter just killed its new mature content rules. 'Honestly? We botched it.'",
                "Swatch launch sparks 'chaotic' scenes and store closures. Some shoppers have been trying to resell a new line of watches priced at £335 each for up to £16,000.",
            ],
            &[
                "Consumer shopping guides, retail sale deals, personal lifestyle, leisure, hobbies, sports matches, fictional entertainment, technology, ai, llm.",
                "Microsoft Surface Laptop 8 review: Fast Panther Lake power, shocking price. The Microsoft Surface Laptop 8 offers strong performance and long battery life. However, its high $1,949 asking price is a lot to ask for the average consumer.",
            ],
        ),
        Category::Politics => (
            &[
                "Politics. Government legislation, election, policy, geopolitics, foreign-affairs.",
                "Keir Starmer faces rebellion over welfare cuts as 50 Labour MPs threaten to vote against budget. The government is scrambling to avoid a humiliating defeat in the Commons.",
                "Trump signs executive order suspending military aid to Ukraine. The White House said the decision reflected a new approach to ending the conflict in eastern Europe.",
                "HS2 could cost up to £102.7bn and trains will be slower than first planned. The new cost range and train speed are being announced as a 'reset' of the delayed, over-budget and vastly scaled-back project is carried out.",
                "Disney erased FiveThirtyEight. Nothing on the internet lasts forever. But Disney’s 10-year mismanagement of FiveThirtyEight is its own story.",
                "Rhun ap Iorwerth says he raised independence with Starmer. Rhun ap Iorwerth was accused of being distracted by the issue after he pressed the case for more powers.",
                "Has anybody seen Nigel? Speculation swirls as Farage performs disappearing act. As campaigners take to the streets for what could be the most significant byelection for decades, the Reform leader’s absence remains a mystery",
                "Andy Burnham Confirmed As Labour Candidate In Crunch Makerfield By-Election. He was the only person to throw their hat into the ring.",
            ],
            &[
                "Technology gadgets, software programming tutorials, developer content, legal regulatory compliance, crime allegations, product reviews, individual scientific discoveries.",
            ],
        ),
        Category::Law => (
            &[
                "Law. Court litigation, legal systems, criminal justice, regulatory compliance, content regulation, hate crime law, civil rights violations, stolen, thieves, victims, emergency services, police, murder, violence, assault, theft.",
                "Supreme Court rules government's Rwanda deportation policy unlawful. The unanimous judgment found the plan breached the UK's international human rights obligations.",
                "PSNI data breach: Staff paid £40m in compensation. Almost £40 million pounds has been paid out to more than 5000 police officers and civilian staff impacted by a 2023 data breach.",
                "Melrose twins cause double takes when called out to emergencies. Identical twins Lee and Liam Myers both work with the emergency services in the Scottish Borders.",
            ],
            &["Entertainment news, celebrity, fashion trends, food recipes, travel."],
        ),
        Category::Health => (
            &[
                "Health. Medical research, healthcare systems, mental wellness, disease prevention, clinical treatments, meningitis, infection spread, virus, bacteria, covid-19, epidemic, pandemic, hospital, ambulance.",
                "New Alzheimer's drug slows cognitive decline by 35% in landmark clinical trial. The treatment could be approved by regulators as early as next year.",
                "Measles cases in England reach highest level in two decades as vaccine uptake falls. Health officials are urging parents to ensure their children are fully immunised.",
                "Disabled man 'rotting' on a hospital ward - despite being told he is fit to go home. Ravi Mehta says his care could be terminated if he self-discharged. His NHS care board says its decisions have not been driven by costs.",
                "Soldier on parachuting to Tristan da Cunha for Hantavirus aid. A specialist team helped supply critical medical support to Tristan da Cunha in the South Atlantic.",
            ],
            &[
                "Politics, government spending, corporate business, technology hardware, software, criminal assault.",
            ],
        ),
        Category::Culture => (
            &[
                "Culture. Arts, entertainment, celebrities, movies, history.",
                "Netflix cancels Stranger Things spinoff after poor viewing figures. The streaming giant confirmed the decision just three months after the show's world premiere.",
                "Billie Eilish announces world tour in support of her third studio album, with 60 dates across North America, Europe, and Asia. Tickets go on sale Friday.",
                "King Charles and Queen Camilla are visiting Belfast. First stop for the royal couple was an event in Belfast to celebrate the city hosting this year's Irish music festival Fleadh Cheoil na hÉireann",
                "3 new to Prime Video thriller movies to watch this week — including a chilling psychological nightmare with a shocking twist (May 19-25). Prime Video is adding a trio of great thriller movies this week, including a twist-fueled watch and this year's Best Picture winner.",
                "Tribute to 'fearless' soldier who died after fall from horse at royal show. Ciara Sullivan's father posted a tribute to his daughter on Facebook on Sunday.",
            ],
            &[
                "Current events, immediate breaking news, product marketing, shopping guides, software.",
            ],
        ),
        Category::Lifestyle => (
            &[
                "Lifestyle. Travel experiences, fashion trends, food recipes, home and garden, outdoor living, personal leisure.",
                "I tried cold water swimming every day for six weeks — here's what it did to my body. From icy lochs to outdoor lidos, I pushed myself through Britain's coldest waters.",
                "The ten best restaurants in London right now, according to our critics. From a revolutionary tasting menu in Mayfair to a neighbourhood gem in Peckham.",
            ],
            &[
                "Industrial hardware, cybersecurity, software tools, digital applications, political policy, legal cases, medical diagnostics, celebrity, technology gadgets.",
            ],
        ),
        Category::Environment => (
            &[
                "Environment. Climate change, ecological conservation, wildlife protection, environmental sustainability.",
                "Record 1.5°C global warming threshold breached for first time in a full calendar year. Scientists say the milestone is a stark warning that Paris Agreement targets are slipping.",
                "UK's wild beaver population hits record high as reintroduction programme transforms river habitats. Conservationists say the animals are returning to rivers they have not inhabited for 400 years.",
                "Inside the secretive and lucrative world of orchid breeding. It can take a decade to bring a new orchid to market, so breeders keep their hi-tech processes secret.",
            ],
            &[
                "Stock market updates, gaming reviews, video streaming, fashion trends, corporate earnings.",
            ],
        ),
        Category::Technology => (
            &[
                "Technology. Artificial intelligence, software programming, cybersecurity, hardware gadgets, creative software tools, digital art applications, 3D modeling, telecommunications, operating system, python, javascript, robot, llm, ai.",
                "Meta launches AI glasses with real-time translation and facial recognition. The second-generation Ray-Ban smart glasses can identify objects and translate speech in 30 languages.",
                "Even More Tagged Union Subsets with Comptime. Type narrowing with switch statements in Zig.",
                "Google Search as you know it is over. Google is transforming Search from a list of links into an AI-powered experience filled with conversational answers, autonomous agents, and interactive interfaces — a shift that could further reduce traffic to publishers across the web.",
                "I just tested the IdeaPad Slim 5x — and it’s my new favorite affordable laptop for work. Though it might struggle with some demanding tasks, the Lenovo IdeaPad Slim 5x Gen 11 is an efficient, feature-packed laptop that starts at $849.",
                "CISA Admin Leaked AWS GovCloud Keys on Github. Until this past weekend, a contractor for the Cybersecurity & Infrastructure Security Agency (CISA) maintained a public GitHub repository that exposed credentials to several highly privileged AWS GovCloud accounts and a large number of internal CISA systems. Security experts said…",
                "I found 12 must-have PC gaming accessories for building the ultimate setup. It's all fair and well buying the PC to game on, but what about the accessories? Here's my top 12 picks I recommend for creating your at-home setup.",
            ],
            &[
                "Artistic expression, culinary recipes, sports competitions, political history, societal traditions.",
            ],
        ),
        Category::Science => (
            &[
                "Science. Space physics, biological research, academic experiments, scientific discovery.",
                "Satellites and AI used to track UK hedgehogs in bid to slow their population decline. Researchers hope the project will identify barriers preventing hedgehogs from finding food and mates.",
                "James Webb Space Telescope finds evidence of liquid water on exoplanet 120 light years away. The finding raises fresh hopes about the potential for life beyond our solar system.",
                "Why is almost everyone right-handed? The answer may lie in how we learned to walk. Humans are the only primates with a population-wide hand preference. A new Oxford-led study, 'Bipedalism and brain expansion explain human handedness', published in PLOS Biology, traces it back to bipedalism and brain expansion.",
            ],
            &[
                "Corporate sales, marketing promotions, opinion editorials, gaming news, lifestyle tips, crime.",
            ],
        ),
        Category::Sports => (
            &[
                "Sports. Professional athletic teams, tournament matches, league competitions, sporting events.",
                "England beat Australia by three wickets in thrilling Ashes decider at The Oval. Ben Stokes led his side to a famous victory with an unbeaten century in the fourth innings.",
                "Man United sack Erik ten Hag after worst Premier League start in club history. The Dutchman leaves Old Trafford with the team in 15th place after nine games.",
                "How to watch Bournemouth vs Man City: live streams, TV channels for Premier League 25/26 game. We'll show you how to watch Bournemouth vs Man City live streams online and from anywhere, as Pep Guardiola's team aim to keep their Premier League title chances alive",
            ],
            &[
                "Corporate mergers, political legislation, software development, medical research, fine arts, thieves.",
            ],
        ),
    }
}

/// Positive and negative labels for each article type
const fn article_type_labels(
    article_type: ArticleType,
) -> (&'static [&'static str], &'static [&'static str]) {
    match article_type {
        ArticleType::Breaking => (
            &[
                "Breaking News. Live emergency alert, immediate breaking news flash, disaster or crisis happening right now, important vital news.",
                "BREAKING: Major earthquake strikes Turkey, casualties reported. Emergency services are responding to a 7.4 magnitude earthquake that has struck southeastern Turkey.",
                "Flash flooding hits Houston as Storm Harvey makes landfall. Thousands are being evacuated as catastrophic flooding overwhelms emergency services across the city.",
                "John Swinney wins vote to be re-appointed first minister. The SNP leader is to be formally sworn in on Wednesday before he chooses a new cabinet.",
            ],
            &[
                "Video content, watch articles, long-form analysis, historical perspective, personal opinion, product evaluation, humorous, unimportant trivial news.",
                "Has anybody seen Nigel? Speculation swirls as Farage performs disappearing act. As campaigners take to the streets for what could be the most significant byelection for decades, the Reform leader’s absence remains a mystery",
            ],
        ),
        ArticleType::News => (
            &[
                "News. Neutral factual reporting on current events, political developments, official announcements, crime and court case updates, journalistic coverage with attributed sources.",
                "UK saves millions of pounds by ditching Palantir for refugee system. The government said its in-house IT system was more flexible while meeting high standards of security.",
                "Supreme Court rules government's Rwanda deportation policy unlawful. The unanimous judgment found the plan breached the UK's international human rights obligations.",
                "Supermarkets urged to limit food prices by government. Any price caps would be voluntary apply to key groceries such as eggs, bread, and milk, the BBC understands.",
                "Kickstarter just killed its new mature content rules. 'Honestly? We botched it.'",
            ],
            &[
                "Subjective critique, hands-on product testing, marketing sales promotion, personal blog diary.",
            ],
        ),
        ArticleType::Opinion => (
            &[
                "Opinion. Editorial opinion piece, columnist subjective viewpoint, first-person argument.",
                "Chris Mason: Inside the shadow contest to be our next prime minister. It is now likely that we will have another new prime minister, possibly within weeks, our political editor writes.",
                "Why I think the government is wrong on AI regulation. The proposed rules will stifle innovation without meaningfully improving safety, argues our technology columnist.",
                "NADINE DORRIES: Why we must scrap the Online Safety Act I helped bring to life. On Sunday, the home affairs spokesman for Reform UK, Zia Yusuf, posted a video discussing the huge levels of mass migration Britain has had in recent years - and what our party will do about it.",
            ],
            &[
                "Balanced neutral reporting, narrative cultural travel writing, descriptive long-form prose, factual political coverage with multiple perspectives, objective news article, breaking news facts, technical instruction, press release.",
                "Andy Burnham 'to reward' MP who stood aside with top No 10 job. Josh Simons to be offered role of head of policy in Burnham's No 10, Labour sources claim",
            ],
        ),
        ArticleType::Marketing => (
            &[
                "Marketing. Product recommendations, gift guides, affiliate sales links, promotional deals, advertisement, sponsored content.",
                "The best noise-cancelling headphones you can buy right now. We tested dozens of pairs so you don't have to — here are the top picks for every budget.",
                "Amazon Prime Day 2025: the best deals still live right now. Don't miss these incredible discounts on tech, home goods, and more before they sell out.",
            ],
            &[
                "Hard news, investigative reporting, academic research, emergency update, neutral observation.",
            ],
        ),
        ArticleType::Review => (
            &[
                "Review. Critical assessment, hands-on product evaluation, performance rating, service critique, new product review.",
                "2027 Mercedes-Benz CLA 250+ review: I can't believe this luxury EV starts under $50,000. The CLA 250+ shatters the luxury EV mold by delivering flagship tech and class-leading efficiency.",
                "I tried Moft's new MagSafe wallet with Find My support — and its coolest feature turns your iPhone into a tripod. After a week of testing, this is the best MagSafe wallet available.",
                "3 new to Prime Video thriller movies to watch this week — including a chilling psychological nightmare with a shocking twist (May 19-25). Prime Video is adding a trio of great thriller movies this week, including a twist-fueled watch and this year's Best Picture winner.",
            ],
            &[
                "Press release, news announcement, factual update, corporate overview, non-tested speculation.",
            ],
        ),
        ArticleType::Guide => (
            &[
                "Guide. Technical tutorial, step-by-step instructions, how-to walkthrough, setup explainer.",
                "How to set up a VPN on your router: a step-by-step guide. Protect every device in your home with this complete walkthrough for configuring a VPN at the network level.",
                "The complete guide to sourdough bread baking for beginners. Everything you need to know to make your first loaf, from feeding your starter to the perfect bake.",
            ],
            &[
                "Subjective opinion, argumentative piece, breaking news event, entertainment review.",
            ],
        ),
        ArticleType::Feature => (
            &[
                "Feature. Long-form magazine journalism, comprehensive investigative deep dive, analytical piece, narrative cultural travel writing.",
                "Robo-top: The machines that could make your next t-shirt. Most clothes are made in Asia, but new machines could bring some of that work back to the West.",
                "The last coal miners: inside Britain's dying industry. A generation of men watch their way of life disappear as the UK's final deep coal mine prepares to close.",
            ],
            &[
                "Brief factual news report, current-event update, breaking headline, sales advertisement, social media post.",
            ],
        ),
        ArticleType::Post => (
            &[
                "Post. Personal amateur blog post, non-professional individual writing, personal website dev-diary.",
                "Why I quit my job at a FAANG company and moved to a farm. After ten years in tech, I needed a change. Here's what I learned about burnout, identity, and slow living.",
                "I switched to Linux full-time for a month — here's what happened. After years of using Windows, I took the plunge and documented every frustration and surprise along the way.",
            ],
            &[
                "News article, journalistic report, official political announcement, magazine feature, academic research, corporate whitepaper.",
            ],
        ),
        ArticleType::Video => (
            &[
                "Video. YouTube visual media, video essay, film documentary, online streaming content.",
                "Watch: How a medieval cathedral is being restored stone by stone. This stunning short film documents the decade-long restoration of Notre-Dame de Paris by master craftsmen.",
                "Video: Inside Ukraine's drone factory that's changing the war. Footage from a secret facility shows the scale of Ukraine's domestic weapons production effort.",
                "Watch: The day Labour's potential leadership race began to heat up. Political turmoil continued in Westminster on Thursday after Health Secretary Wes Streeting resigned Starmer's government saying he had 'lost confidence' in his leadership.",
            ],
            &[
                "Written text article listing shows or episodes to watch, streaming content guide, formal editorial, technical whitepaper, email newsletter.",
                "NADINE DORRIES: Why we must scrap the Online Safety Act I helped bring to life. On Sunday, the home affairs spokesman for Reform UK, Zia Yusuf, posted a video discussing the huge levels of mass migration Britain has had in recent years - and what our party will do about it.",
            ],
        ),
    }
}

// How strongly the pair classifier adjusts the raw cosine similarity.
const MERGE_SIGNAL_WEIGHT: f32 = 0.25;

// Pairs of ("Title A. Summary A.", "Title B. Summary B.") used to build merge centroids.
const MERGE_POSITIVE_PAIRS: &[(&str, &str)] = &[(
    "Reform's Makerfield candidate said King should open up Buckingham Palace for asylum seekers. Robert Kenyon wrote about Asian men 'assaulting white people en masse' and 'an invasion of foreign criminals' on his now suspended X account.",
    "Reform UK Makerfield candidate was 'friends' with neo-fascist leader. REFORM UK’s candidate in the Makerfield by-election was formerly “friends” on Facebook with a neo-fascist leader",
)];

const MERGE_NEGATIVE_PAIRS: &[(&str, &str)] = &[
    (
        "Reform UK Makerfield candidate was 'friends' with neo-fascist leader. REFORM UK’s candidate in the Makerfield by-election was formerly “friends” on Facebook with a neo-fascist leader",
        "Reform UK Councillors Told to Speak Only to GB News’ as Party Shuns Local Press. Local news editor in Ipswich says Reform party chair instructed his councillors to shut out the press in favour of only speaking to the right-wing news channel",
    ),
    (
        "NADINE DORRIES: Why we must scrap the Online Safety Act I helped bring to life. On Sunday, the home affairs spokesman for Reform UK, Zia Yusuf, posted a video discussing the huge levels of mass migration Britain has had in recent years - and what our party will do about it.",
        "Reform's Makerfield candidate said King should open up Buckingham Palace for asylum seekers. Robert Kenyon wrote about Asian men 'assaulting white people en masse' and 'an invasion of foreign criminals' on his now suspended X account.",
    ),
];

static EMBEDDING_MODEL: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    EMBEDDING_MODEL.lock().await.embed(texts, None)
}

pub async fn generate_article_embeddings(articles: &[ArticleSource]) -> Result<Vec<Vec<f32>>> {
    let texts = articles
        .iter()
        .map(|s| format!("{}. {}", s.title, s.summary))
        .collect::<Vec<_>>();

    let mut embs = generate_embeddings(&texts).await?;
    for emb in &mut embs {
        normalize(emb);
    }
    Ok(embs)
}

fn normalize(v: &mut Vec<f32>) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v {
            *x /= norm;
        }
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn hadamard_normalize(a: &[f32], b: &[f32]) -> Vec<f32> {
    let mut h: Vec<f32> = a.iter().zip(b).map(|(x, y)| x * y).collect();
    normalize(&mut h);
    h
}

fn average_embeddings(groups: &[&[Vec<f32>]]) -> Vec<Vec<f32>> {
    groups
        .iter()
        .map(|group| {
            if group.is_empty() {
                return vec![];
            }
            let dim = group[0].len();
            let mut avg = vec![0.0f32; dim];
            for e in *group {
                for (a, b) in avg.iter_mut().zip(e) {
                    *a += b;
                }
            }
            let n = group.len() as f32;
            for x in &mut avg {
                *x /= n;
            }
            normalize(&mut avg);
            avg
        })
        .collect()
}

struct LabelEmbeddings {
    cat_pos: Vec<Vec<f32>>,
    cat_neg: Vec<Vec<f32>>,
    type_pos: Vec<Vec<f32>>,
    type_neg: Vec<Vec<f32>>,
}

static LABEL_EMBEDDINGS: OnceCell<LabelEmbeddings> = OnceCell::const_new();

pub async fn classify(embedding: &[f32]) -> Result<(ArticleType, Category)> {
    let labels = LABEL_EMBEDDINGS
        .get_or_try_init(|| async {
            let mut all_texts: Vec<String> = Vec::new();
            let mut slices: [Vec<std::ops::Range<usize>>; 4] = Default::default();

            let mut push = |phrases: &[&str], slot: &mut Vec<std::ops::Range<usize>>| {
                let start = all_texts.len();
                all_texts.extend(phrases.iter().map(std::string::ToString::to_string));
                slot.push(start..all_texts.len());
            };

            for c in Category::iter() {
                let (pos, neg) = category_labels(c);
                push(pos, &mut slices[0]);
                push(neg, &mut slices[1]);
            }
            for a in ArticleType::iter() {
                let (pos, neg) = article_type_labels(a);
                push(pos, &mut slices[2]);
                push(neg, &mut slices[3]);
            }

            let embs = generate_embeddings(&all_texts).await?;
            let centroid = |s: &[std::ops::Range<usize>]| {
                average_embeddings(&s.iter().map(|r| &embs[r.clone()]).collect::<Vec<_>>())
            };
            let [cp, cn, tp, tn] = slices;
            anyhow::Ok(LabelEmbeddings {
                cat_pos: centroid(&cp),
                cat_neg: centroid(&cn),
                type_pos: centroid(&tp),
                type_neg: centroid(&tn),
            })
        })
        .await?;

    let score = |pos: &[f32], neg: &[f32]| {
        cosine_similarity(embedding, pos) - cosine_similarity(embedding, neg)
    };
    let best_cat = Category::iter()
        .zip(labels.cat_pos.iter().zip(&labels.cat_neg))
        .map(|(v, (pos, neg))| (v, score(pos, neg)))
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(v, _)| v)
        .unwrap();

    let best_type = ArticleType::iter()
        .zip(labels.type_pos.iter().zip(&labels.type_neg))
        .map(|(v, (pos, neg))| (v, score(pos, neg)))
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(v, _)| v)
        .unwrap();

    Ok((best_type, best_cat))
}

struct MergeCentroids {
    positive: Vec<f32>,
    negative: Vec<f32>,
}

static MERGE_CENTROIDS: OnceCell<MergeCentroids> = OnceCell::const_new();

/// Returns a similarity score adjusted by the merge pair classifier.
pub async fn merge_similarity(emb_a: &[f32], emb_b: &[f32]) -> f32 {
    let sim = cosine_similarity(emb_a, emb_b);

    let centroids = MERGE_CENTROIDS
        .get_or_try_init(|| async {
            let all_texts: Vec<String> = MERGE_POSITIVE_PAIRS
                .iter()
                .chain(MERGE_NEGATIVE_PAIRS.iter())
                .flat_map(|(a, b)| [a.to_string(), b.to_string()])
                .collect();

            if all_texts.is_empty() {
                return anyhow::Ok(MergeCentroids {
                    positive: vec![],
                    negative: vec![],
                });
            }

            let embs = generate_embeddings(&all_texts).await?;
            let n_pos = MERGE_POSITIVE_PAIRS.len();

            let mut pos_sum = vec![0.0f32; embs[0].len()];
            for i in 0..n_pos {
                let h = hadamard_normalize(&embs[i * 2], &embs[i * 2 + 1]);
                for (a, b) in pos_sum.iter_mut().zip(&h) {
                    *a += b;
                }
            }

            let mut neg_sum = vec![0.0f32; embs[0].len()];
            let neg_offset = n_pos * 2;
            for i in 0..MERGE_NEGATIVE_PAIRS.len() {
                let h =
                    hadamard_normalize(&embs[neg_offset + i * 2], &embs[neg_offset + i * 2 + 1]);
                for (a, b) in neg_sum.iter_mut().zip(&h) {
                    *a += b;
                }
            }

            normalize(&mut pos_sum);
            normalize(&mut neg_sum);
            anyhow::Ok(MergeCentroids {
                positive: pos_sum,
                negative: neg_sum,
            })
        })
        .await;

    let Ok(centroids) = centroids else {
        return sim;
    };
    if centroids.positive.is_empty() {
        return sim;
    }

    let h = hadamard_normalize(emb_a, emb_b);
    let signal =
        cosine_similarity(&h, &centroids.positive) - cosine_similarity(&h, &centroids.negative);
    MERGE_SIGNAL_WEIGHT.mul_add(signal, sim)
}
