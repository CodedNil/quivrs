use crate::shared::{ArticleSource, Category};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::sync::LazyLock;
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, OnceCell};

// --- Label writing guide ---
//
// Labels are embedded with EmbeddingGemma 300M and classified via cosine similarity.
// The label embedding is a mean-pool over all its token embeddings, so every token
// contributes equally. Write labels as dense keyword lists, not prose.
//
// Rules:
//   - Use single distinctive tokens. "jailed" beats "sentenced to prison".
//   - Prefer named things: brands, people, places, organisations (NHS, Oscars, Starmer).
//     They are uniquely associated with one category and carry strong signal.
//   - Avoid tokens that appear across multiple labels — they dilute discrimination.
//   - No negations: "not marketing" adds "marketing" semantics to the vector.
//   - No filler prose: "Includes coverage of" wastes tokens on noise.
//   - Multi-word phrases are fine when the compound is widely known (Premier League,
//     Black Friday), but single tokens are preferred where one exists.
//   - Commas make no difference to the embedding — they are just punctuation tokens.
// ---

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const MODEL_NAME: &str = "EmbeddingGemma300M";

pub const fn category_label(category: Category) -> &'static str {
    match category {
        // Corporate finance, stock markets, and macroeconomics
        Category::Business => {
            "earnings revenue profit stocks shares merger acquisition IPO dividends CEO layoffs tariffs GDP inflation hedge fund private equity startup venture capital fiscal bonds FTSE Nasdaq shareholder buyout"
        }
        // Government, parliament, elections, and political parties
        Category::Politics => {
            "parliament MPs chancellor legislation elections Conservative Labour LibDems Reform cabinet minister Starmer Farage Reeves Sunak vote constituency manifesto polling Holyrood Whitehall HS2 sanctions diplomacy PMQs SNP"
        }
        // Courts, crime, police investigations, and criminal justice
        Category::Law => {
            "court trial convicted jailed sentenced defendant prosecution verdict jury judge murder stabbing knife beating assault attack grooming exploitation abuse offender indecent coercive stalking fraud theft robbery ASA FCA banned misconduct DWP lawsuit acquitted injunction plaintiff caution probe"
        }
        // Clinical medicine and personal health — NHS, diagnosis, treatment, fitness, diet
        Category::Health => {
            "NHS hospital GP vaccine diagnosis cancer surgery prescription clinical trial symptoms disease epidemic inpatient psychiatric pharmaceutical medication therapy chronic disability overdose ward nurse stroke dementia autism mental health fitness diet nutrition exercise workout weight sleep supplements vitamins"
        }
        // Entertainment, film, TV, music, celebrities, and streaming
        Category::Culture => {
            "Netflix Disney streaming celebrities actor director Oscars BAFTA album concert band horror thriller documentary royals entertainment Grammy Glastonbury blockbuster sitcom season episode premiere trailer how to watch film"
        }
        // Domestic life, cooking, home, fashion, and consumer tips
        Category::Lifestyle => {
            "recipe cooking cleaning decor fashion wardrobe travel garden kitchen wedding household mattress bedding IKEA interior skincare bathroom laundry storage hacks tips ants vinegar coffee"
        }
        // Climate, ecology, wildlife, and the natural world
        Category::Environment => {
            "climate carbon emissions wildlife biodiversity deforestation flooding drought renewable ecology extinction conservation glaciers methane solar wind reef hedgehog farming heatwave COP rewilding pollution"
        }
        // Software, hardware, AI, coding, and the tech industry
        Category::Technology => {
            "software hardware AI ChatGPT Claude Gemini smartphone app coding Python Ruby Rust JavaScript cybersecurity breach hacking VPN GPU API programming GitHub Nintendo SpaceX rocket satellite smartwatch wearable console headphones earphones audio speakers display OLED television firmware open-source SDK"
        }
        // Scientific research, astronomy, biology, and academic discovery
        Category::Science => {
            "astronomy telescope genome species paleontology neuroscience botany orchid physics quantum biology chemistry fossils stargazing celestial earthshine mathematics NASA experiment journal hypothesis lab discovery space science"
        }
        // Physical competitive sports and athletic leagues
        Category::Sports => {
            "football cricket tennis rugby golf boxing swimming cycling wicket tournament Premier League Champions League Europa League transfer squad batting medal athletics Wimbledon Olympics F1 Grand Prix penalty fixture goalscorer hat-trick"
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

static LABEL_EMBEDDINGS: OnceCell<Vec<Vec<f32>>> = OnceCell::const_new();

pub async fn seed_label_cache(cat: Vec<Vec<f32>>) {
    let _ = LABEL_EMBEDDINGS.set(cat);
}

pub async fn classify(embedding: &[f32]) -> Result<Category> {
    let labels = LABEL_EMBEDDINGS
        .get_or_try_init(|| async {
            let texts: Vec<String> = Category::iter()
                .map(|c| format!("title: none | text: {}", category_label(c)))
                .collect();
            anyhow::Ok(embed_label_texts(&texts).await?)
        })
        .await?;

    let best_cat = Category::iter()
        .zip(labels)
        .max_by(|(_, a), (_, b)| {
            cosine_similarity(embedding, a).total_cmp(&cosine_similarity(embedding, b))
        })
        .map(|(v, _)| v)
        .unwrap();

    Ok(best_cat)
}
