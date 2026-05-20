use crate::shared::{ArticleSource, ArticleType, Category};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::LazyLock;
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, OnceCell};

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const MODEL_NAME: &str = "EmbeddingGemma300M";

const fn category_label(category: Category) -> &'static str {
    match category {
        Category::Business => {
            "Business, corporate finance, and global macroeconomics. Includes corporate earnings reports, financial market updates, stock trading, mergers and acquisitions, startup funding, venture capital, government economic policy, inflation, GDP, central bank decisions, trade agreements, layoffs, executive appointments, and industry competition."
        }
        Category::Politics => {
            "Politics, governmental systems, and geopolitical affairs. Includes coverage of elections, legislation, political party dynamics, taxes, policymaking, legislative updates from parliament or congress, global diplomacy, foreign affairs, international relations, political appointments, and campaigns."
        }
        Category::Law => {
            "Law, crime, the legal system, and criminal justice. Includes coverage of court cases, criminal trials, verdicts, sentencing, police investigations, arrests, civil lawsuits, judicial rulings, regulatory enforcement, fraud, theft, violence, cybercrime, and civil rights violations."
        }
        Category::Health => {
            "Health, medicine, and healthcare infrastructure. Includes medical research breakthroughs, clinical trial results, drug or treatment approvals, mental health, public health campaigns, disease outbreaks, epidemics, vaccines, nutrition science, and patient stories."
        }
        Category::Culture => {
            "Culture, media, and the entertainment industry. Includes television shows, streaming content, series releases, true crime dramas, documentaries, films, music, and theatre. Covers celebrity updates, royal family news, human interest stories, book releases, art exhibitions, and media broadcasting guides explaining how to watch or stream a show."
        }
        Category::Lifestyle => {
            "Lifestyle, consumer shopping, retail discounts, products, and home life. Focuses on interior organization, home decor, furniture, kitchen storage solutions, fashion design, weddings, and consumer product deals. Covers popular trend items, travel, food, and family relationships."
        }
        Category::Environment => {
            "The environment, nature, and ecological changes. Includes reporting on climate change, global warming, carbon emissions, renewable energy transitions, wildlife conservation, biodiversity, deforestation, ocean pollution, ecological research, and green sustainability initiatives."
        }
        Category::Technology => {
            "Consumer technology, consumer electronics, software applications, streaming platforms, apps, hardware, gadgets, and the tech industry. Includes tech product pricing changes, subscriptions, smartphones, operating systems, social media platforms, and big tech announcements. Also explicitly covers cybersecurity incidents, data breaches, domain abuse, online scams, hacking, vulnerabilities, artificial intelligence, and machine learning."
        }
        Category::Science => {
            "Fundamental science, academic research, and discovery. Includes coverage of space exploration, astronomy, physics, biology, chemistry, genetics, neuroscience, paleontology, academic studies, and peer-reviewed scientific findings."
        }
        Category::Sports => {
            "Sports, athletics, and professional competitive leagues. Includes coverage of football, cricket, tennis, rugby, cycling, golf, boxing, track and field, and swimming. Tracks tournament progress, match results, player transfers, and sporting records."
        }
    }
}

const fn article_type_labels(article_type: ArticleType) -> &'static str {
    match article_type {
        ArticleType::Breaking => {
            "Fast-breaking news alert, immediate live update, or urgent news flash reporting on a developing crisis, natural disaster, or major public emergency happening right now."
        }
        ArticleType::News => {
            "Standard current events and objective news reporting. Documents recent public incidents, odd or unusual current events, professional bans, employment dismissals, official announcements, local developments, or public records. Factual, informational journalistic coverage of daily events."
        }
        ArticleType::Opinion => {
            "Opinion piece, editorial column, or subjective essay presenting an individual's personal argument or commentary, often utilizing first-person perspectives to critique public affairs."
        }
        ArticleType::Marketing => {
            "Marketing post, sponsored piece, or promotional advertisement intended to drive consumer sales. Features commercial product recommendations, gift guides, affiliate purchase links, or shopping deals."
        }
        ArticleType::Review => {
            "Critical review or performance evaluation of a commercial product, device, creative work, or consumer service. Includes hands-on assessment, technical analysis, and explicit pros, cons, or quality ratings."
        }
        ArticleType::Guide => {
            "Technical tutorial, step-by-step instructional guide, or educational how-to walkthrough designed to explain operational setups, troubleshoot technical issues, or teach a specific skill."
        }
        ArticleType::Feature => {
            "Long-form narrative storytelling or a deep-dive piece of investigative journalism. Uses magazine-style writing to explore a profile, investigate a complex social issue, or provide an extensive analytical overview."
        }
        ArticleType::Post => {
            "Casual, informal personal blog post, personal journal entry, or individual developer diary, typical of an amateur or independent hobbyist rather than a professional news agency."
        }
        ArticleType::Video => "Video container, multimedia player embed, or video post.",
    }
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

pub async fn generate_article_embeddings(
    articles: &[ArticleSource],
) -> Result<(Vec<Vec<f32>>, Vec<Vec<f32>>)> {
    let similarity_texts = articles.iter().map(|s| {
        format!(
            "task: sentence similarity | query: {}. {}",
            s.title, s.summary
        )
    });
    let classification_texts = articles
        .iter()
        .map(|s| format!("task: classification | query: {}. {}", s.title, s.summary));
    let texts: Vec<String> = similarity_texts.chain(classification_texts).collect();

    let mut embs = generate_embeddings(&texts).await?;
    for emb in &mut embs {
        normalize(emb);
    }
    let classification = embs.split_off(articles.len());
    Ok((embs, classification))
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

pub async fn classify(embedding: &[f32]) -> Result<(ArticleType, Category)> {
    let labels = LABEL_EMBEDDINGS
        .get_or_try_init(|| async {
            let texts: Vec<String> = Category::iter()
                .map(|c| format!("title: none | text: {}", category_label(c)))
                .chain(
                    ArticleType::iter()
                        .map(|a| format!("title: none | text: {}", article_type_labels(a))),
                )
                .collect();

            let n_cat = Category::iter().count();
            let mut embs = generate_embeddings(&texts).await?;
            for emb in &mut embs {
                normalize(emb);
            }
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
