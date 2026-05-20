use crate::shared::{ArticleSource, ArticleType, Category};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::sync::LazyLock;
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, OnceCell};

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const MODEL_NAME: &str = "EmbeddingGemma300M";

pub const fn category_label(category: Category) -> &'static str {
    match category {
        Category::Business => {
            "Business, corporate finance, and private sector commercial activity. Includes corporate earnings reports, financial market updates, stock trading, mergers and acquisitions, startup funding, venture capital, inflation, GDP, central bank decisions, trade agreements, layoffs, executive appointments, and industry competition."
        }
        Category::Politics => {
            "Politics, governmental systems, and geopolitical affairs. Includes coverage of elections, legislation, political party dynamics, taxes, policymaking, parliament and congress updates, global diplomacy, international relations, sanctions, political appointments, campaigns, government infrastructure projects such as rail, roads, and HS2, public spending decisions, senior politicians' actions, statements, resignations, and personal financial disclosures or conduct scandals involving elected politicians or party figures."
        }
        Category::Law => {
            "Law, crime, the legal system, and criminal justice. Includes coverage of court cases, criminal trials, verdicts, sentencing, police investigations including investigations into suspicious deaths and unexplained drownings, arrests, civil lawsuits, judicial rulings, regulatory enforcement, fraud, theft, violence, cybercrime, financial fraud, insurance fraud, benefit fraud, consumer protection enforcement, actions by financial watchdogs such as the FCA or FTC, civil rights violations, professional misconduct hearings, bans from professions or workplaces, fitness-to-plead determinations, and investigations into institutional wrongdoing."
        }
        Category::Health => {
            "Clinical health, medicine, and medical treatment. Includes medical research breakthroughs, pharmaceutical drug trials and study results, drug or treatment approvals, clinical psychiatric care, serious mental illness diagnoses and treatment, disease outbreaks, epidemics, vaccines, NHS hospital patient services, NHS and healthcare system policy decisions about patient care, access to medical treatment, and personal accounts of living with diagnosed chronic illness or serious medical conditions. Focused on the clinical practice of medicine and treatment of diagnosed illness and medical conditions."
        }
        Category::Culture => {
            "Culture, media, and the entertainment industry. Includes television shows, streaming content, series releases, crime fiction and drama series, documentaries, films, music genres, bands, theatre, and video game culture and gaming entertainment. Covers celebrity entertainment news, royal family official state visits and public appearances, book releases, art exhibitions, music awards and festivals, and media broadcasting guides explaining how to watch or stream a show."
        }
        Category::Lifestyle => {
            "Lifestyle, domestic home life, and practical household living. Includes household cleaning tips, home decor, interior organisation, kitchen storage, cooking recipes, food, fashion and clothing, weddings, travel, family household relationships, and personal employment challenges or job searching. Also covers consumer shopping for household goods, retail discounts on everyday items, and product deals for home and personal life."
        }
        Category::Environment => {
            "The natural environment, ecology, and physical world. Includes reporting on climate change, global warming, carbon emissions, renewable energy transitions, wildlife conservation, biodiversity, deforestation, ocean pollution, weather extremes, ecological research, and green sustainability initiatives. Focused on the natural physical world: land, sea, air, plants, and animals."
        }
        Category::Technology => {
            "Consumer technology, consumer electronics, software, and the tech industry. Includes smartphones, computers, operating systems, apps, streaming platforms, hardware, gadgets, wearable technology, smartwatches, fitness trackers, wireless headphones, earphones, audio equipment, video games, gaming consoles, game controllers, and gaming hardware. Also covers programming languages, software development, coding tools, developer retrospectives, personal programming essays, independent hobbyist coding projects, developer blog posts about Rust, Ruby, Python, C, Go, and other languages, tech company news, product launches and platform feature announcements, AI assistants and large language models including ChatGPT, Claude, and Gemini, machine learning, cybersecurity incidents, data breaches, hacking, and vulnerabilities."
        }
        Category::Science => {
            "Fundamental science, academic research, and discovery. Includes astronomy, celestial events, natural sky phenomena, stargazing and sky-watching, space exploration, physics, biology, chemistry, botany including plant cultivation and orchid and flower breeding, horticulture and garden science, genetics, neuroscience, paleontology, ecology, natural history, and peer-reviewed scientific findings. Covers in-depth features on natural processes, plant and animal biology, and scientific curiosities."
        }
        Category::Sports => {
            "Physical sports, athletics, and professional competitive athletic leagues. Includes coverage of football, cricket, tennis, rugby, cycling, golf, boxing, track and field, swimming, and other real-world competitive sports. Tracks tournament progress, match results, player transfers, and sporting records."
        }
    }
}

pub const fn article_type_label(article_type: ArticleType) -> &'static str {
    match article_type {
        ArticleType::Breaking => {
            "Breaking news alert published the moment a physical emergency is confirmed, while emergency services are actively responding. Covers incidents where police are attending a stabbing or shooting with confirmed injuries, paramedics responding to a drowning, firefighters on scene at a building fire with casualties, or an explosion with confirmed victims. Defined by real-time emergency services response and immediate confirmed harm at the moment of publication."
        }
        ArticleType::News => {
            "Standard current events and objective news reporting. Documents recent public incidents, crime reports, court case summaries, human interest news items, odd or unusual current events, professional bans, employment dismissals, official announcements, local developments, political developments, infrastructure updates, public records, and newly published scientific study results or health research findings. Factual, informational journalistic coverage of daily events."
        }
        ArticleType::Opinion => {
            "Opinion piece, editorial column, or subjective essay presenting an individual's personal argument or commentary, often utilizing first-person perspectives to critique public affairs."
        }
        ArticleType::Marketing => {
            "Marketing post, sponsored piece, or affiliate content written to sell specific retail products to readers. Includes seasonal retail sale roundups with product prices and discount savings, shopping gift guides with buy links, affiliate product lists with purchase recommendations, and sponsored product promotions. The article earns commission from or is paid to promote specific products for purchase."
        }
        ArticleType::Review => {
            "Critical hands-on review or product evaluation where the reviewer personally tests a commercial product, device, consumer service, or creative work. Includes personal testing reports, head-to-head device comparisons, technical performance analysis, and articles that give explicit pros, cons, star ratings, or verdict scores. The reviewer has directly used or evaluated the subject."
        }
        ArticleType::Guide => {
            "Step-by-step instructional article or practical how-to guide designed to teach the reader how to accomplish something. Includes technical programming tutorials, debugging walkthroughs, household cleaning and maintenance instructions, cooking recipes, DIY and home improvement guides, and practical how-to explainers. The article teaches a concrete skill or practical process."
        }
        ArticleType::Feature => {
            "Extended magazine-style longread or in-depth investigative journalism going well beyond a news summary. Multi-source reporting, deep character profiles, or sustained analytical exploration of a complex issue. Clearly a standalone longread."
        }
        ArticleType::Post => {
            "Casual first-person blog post on an individual's personal website, developer blog, or independent technical newsletter. The author writes about their own code, technical project, or experience with a specific technology — for example: 'I built this tool and here is how it works', 'I worked on this problem', 'here is my experience with this library'. Written by individual developers, engineers, or technical hobbyists about their own work."
        }
    }
}

pub fn label_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(b"\0");
    hasher.update(MODEL_NAME.as_bytes());
    hasher.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

static EMBEDDING_MODEL: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    EMBEDDING_MODEL.lock().await.embed(texts, None)
}

pub fn article_text(s: &ArticleSource) -> String {
    [
        s.title.replace('\n', " "),
        s.tags.iter().take(8).join(", ").replace('\n', " "),
        s.summary
            .chars()
            .take(500)
            .collect::<String>()
            .replace('\n', " "),
    ]
    .iter()
    .filter(|field| !field.is_empty())
    .join(". ")
}

pub async fn generate_article_embeddings(articles: &[ArticleSource]) -> Result<Vec<Vec<f32>>> {
    let texts: Vec<String> = articles.iter().map(article_text).collect();
    let mut embs = generate_embeddings(&texts).await?;
    for emb in &mut embs {
        normalize(emb);
    }
    Ok(embs)
}

pub async fn embed_label_texts(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    let mut embs = generate_embeddings(texts).await?;
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

struct LabelEmbeddings {
    cat: Vec<Vec<f32>>,
    type_: Vec<Vec<f32>>,
}

static LABEL_EMBEDDINGS: OnceCell<LabelEmbeddings> = OnceCell::const_new();

pub async fn seed_label_cache(cat: Vec<Vec<f32>>, type_: Vec<Vec<f32>>) {
    let _ = LABEL_EMBEDDINGS.set(LabelEmbeddings { cat, type_ });
}

pub async fn classify(embedding: &[f32]) -> Result<(ArticleType, Category)> {
    let labels = LABEL_EMBEDDINGS
        .get_or_try_init(|| async {
            let n_cat = Category::iter().count();
            let texts: Vec<String> = Category::iter()
                .map(|c| format!("title: none | text: {}", category_label(c)))
                .chain(
                    ArticleType::iter()
                        .map(|a| format!("title: none | text: {}", article_type_label(a))),
                )
                .collect();
            let mut embs = embed_label_texts(&texts).await?;
            let type_ = embs.split_off(n_cat);
            anyhow::Ok(LabelEmbeddings { cat: embs, type_ })
        })
        .await?;

    let best_cat = Category::iter()
        .zip(&labels.cat)
        .max_by(|(_, a), (_, b)| {
            cosine_similarity(embedding, a).total_cmp(&cosine_similarity(embedding, b))
        })
        .map(|(v, _)| v)
        .unwrap();

    let best_type = ArticleType::iter()
        .zip(&labels.type_)
        .max_by(|(_, a), (_, b)| {
            cosine_similarity(embedding, a).total_cmp(&cosine_similarity(embedding, b))
        })
        .map(|(v, _)| v)
        .unwrap();

    Ok((best_type, best_cat))
}
