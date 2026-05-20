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
            "Politics, governmental systems, and geopolitical affairs. Includes coverage of elections, legislation, political party dynamics, taxes, policymaking, parliament and congress updates, global diplomacy, international relations, sanctions, political appointments, campaigns, government infrastructure projects, public spending decisions, government contracts with private companies, and senior politicians' actions, statements, and resignations."
        }
        Category::Law => {
            "Law, crime, the legal system, and criminal justice. Includes coverage of court cases, criminal trials, verdicts, sentencing, police investigations, arrests, civil lawsuits, judicial rulings, regulatory enforcement, fraud, theft, violence, cybercrime, civil rights violations, professional misconduct hearings, bans from professions or workplaces, fitness-to-plead determinations, and investigations into institutional wrongdoing."
        }
        Category::Health => {
            "Clinical health, medicine, and public healthcare. Includes medical research breakthroughs, pharmaceutical and medical study results, drug or treatment approvals, clinical psychiatric care and serious mental illness, disease outbreaks, epidemics, vaccines, hospital and healthcare policy, and personal accounts of living with chronic illness or serious long-term medical conditions."
        }
        Category::Culture => {
            "Culture, media, and the entertainment industry. Includes television shows, streaming content, series releases, true crime dramas, documentaries, films, music, and theatre. Covers celebrity updates, royal family news, human interest stories, book releases, art exhibitions, and media broadcasting guides explaining how to watch or stream a show."
        }
        Category::Lifestyle => {
            "Lifestyle, domestic home life, and practical household living. Includes household cleaning tips, home decor, interior organisation, kitchen storage, cooking recipes, food, fashion and clothing, weddings, travel, and family household relationships. Also covers consumer shopping for household goods, retail discounts on everyday items, and product deals for home and personal life."
        }
        Category::Environment => {
            "The environment, nature, and ecological changes. Includes reporting on climate change, global warming, carbon emissions, renewable energy transitions, wildlife conservation, biodiversity, deforestation, ocean pollution, ecological research, and green sustainability initiatives."
        }
        Category::Technology => {
            "Consumer technology, consumer electronics, software, and the tech industry. Includes smartphones, computers, operating systems, apps, streaming platforms, hardware, gadgets, wearable technology, smartwatches, fitness trackers, wireless headphones, earphones, and audio equipment. Also covers programming languages, software development, coding tools, developer retrospectives, personal programming essays, independent hobbyist coding projects, developer blog posts about Rust, Ruby, Python, C, Go, and other languages, tech company news, product launches and platform feature announcements, AI and machine learning, cybersecurity incidents, data breaches, hacking, and vulnerabilities."
        }
        Category::Science => {
            "Fundamental science, academic research, and discovery. Includes astronomy, celestial events, natural sky phenomena, stargazing and sky-watching, space exploration, physics, biology, chemistry, botany, horticulture, genetics, neuroscience, paleontology, ecology, and peer-reviewed scientific findings. Covers in-depth features on natural processes, plant and animal biology, and scientific curiosities."
        }
        Category::Sports => {
            "Sports, athletics, and professional competitive leagues. Includes coverage of football, cricket, tennis, rugby, cycling, golf, boxing, track and field, and swimming. Tracks tournament progress, match results, player transfers, and sporting records."
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
            "Marketing post, sponsored piece, or promotional advertisement explicitly intended to drive consumer sales. Features commercial product recommendations, curated seasonal shopping collections, deals roundups, gift guides, affiliate purchase links, or product lists with prices and calls to action."
        }
        ArticleType::Review => {
            "Critical review or performance evaluation of a commercial product, device, creative work, or consumer service. Includes hands-on testing, head-to-head product comparisons, technical analysis, and explicit pros, cons, or quality ratings."
        }
        ArticleType::Guide => {
            "Step-by-step instructional guide or practical how-to article designed to teach a skill or explain a process. Includes technical tutorials, debugging walkthroughs, root-cause investigations, household cleaning tips, cooking instructions, home maintenance guides, sky-watching guides, streaming and viewing guides for TV shows and films, best-of picks and top recommendations lists, and how-to explainers across any topic."
        }
        ArticleType::Feature => {
            "Extended magazine-style longread or in-depth investigative journalism going well beyond a news summary. Multi-source reporting, deep character profiles, or sustained analytical exploration of a complex issue. Clearly a standalone longread."
        }
        ArticleType::Post => {
            "Casual, informal first-person personal blog post or personal journal entry, written by one individual sharing their own experience, hobby project, or personal thoughts. Typical of personal websites, indie developer blogs, or independent newsletters where the author writes about their own life or project in their own voice."
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
