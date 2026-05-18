use crate::shared::{ArticleType, Category};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::LazyLock;
use strum::{EnumCount, IntoEnumIterator};
use tokio::sync::{Mutex, OnceCell};

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const MODEL_NAME: &str = "EmbeddingGemma300M";

static STATE: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

pub async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    STATE.lock().await.embed(texts, None)
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

const fn category_label(category: Category) -> &'static str {
    match category {
        Category::Business => {
            "Corporate business economy finance commerce startups markets companies investments earnings"
        }
        Category::Politics => {
            "Government elections public policy geopolitics politicians democracy foreign affairs public sector spending government decisions ministry"
        }
        Category::Law => {
            "Court cases legislation legal systems crime civil rights justice lawyers lawsuits criminal fined penalty"
        }
        Category::Health => {
            "Medicine healthcare fitness mental wellness hospitals doctors disease treatments medical"
        }
        Category::Entertainment => {
            "Movies TV shows music video games gaming PlayStation Xbox Nintendo consoles celebrity entertainment films actors artists pop culture"
        }
        Category::Culture => {
            "Arts literature history philosophy societal traditions culture museums heritage"
        }
        Category::Lifestyle => {
            "Travel fashion food hobbies daily living recipes cooking personal leisure lifestyle"
        }
        Category::Environment => {
            "Climate conservation sustainability wildlife ecology nature environment emissions"
        }
        Category::Technology => {
            "Software AI hardware gadgets cybersecurity computers programming technology industry telecommunications satellite internet"
        }
        Category::Science => {
            "Space physics biology academic scientific research experiments discoveries"
        }
        Category::Education => {
            "Schools universities teaching educational policy learning students curriculum"
        }
        Category::Sports => {
            "Teams athletes tournaments live sporting events competitions matches leagues professional sports"
        }
    }
}

const fn article_type_label(article_type: ArticleType) -> &'static str {
    match article_type {
        ArticleType::BreakingNews => {
            "Breaking news live emergency disaster crisis unfolding right now urgent real-time"
        }
        ArticleType::News => {
            "News article journalism coverage report story update announcement published by news media outlet press"
        }
        ArticleType::Opinion => "Opinion editorial commentary viewpoint perspective argument",
        ArticleType::Marketing => {
            "Gift guide buying guide product recommendations affiliate shopping best picks sponsored deals promotional advertisement marketing campaign sale roundup"
        }
        ArticleType::Review => "Review product service evaluation rating hands-on assessment",
        ArticleType::Interview => {
            "One-on-one human interview Q&A journalist celebrity exclusive profile spoken conversation transcript sit-down"
        }
        ArticleType::Guide => {
            "Tutorial how-to step-by-step technical instructions walkthrough setup explainer"
        }
        ArticleType::Feature => {
            "Long-form magazine feature investigative reported piece exploring a topic with background historical context"
        }
        ArticleType::Blog => {
            "Personal blog post dev log developer diary podcast audio show individual perspective"
        }
        ArticleType::Newsletter => {
            "Email newsletter subscription substack mailing list company blog direct to subscribers corporate publication"
        }
        ArticleType::Video => {
            "YouTube video essay streaming watch online documentary film visual media embed player"
        }
        ArticleType::Post => {
            "Tweet Twitter X Reddit Instagram TikTok Facebook user-generated status update short message social media account"
        }
    }
}

static LABEL_EMBEDDINGS: OnceCell<Vec<Vec<f32>>> = OnceCell::const_new();

pub async fn classify(embedding: &[f32]) -> Result<(ArticleType, Category)> {
    // Initialize or retrieve cached raw embeddings
    let embs = LABEL_EMBEDDINGS
        .get_or_try_init(|| async {
            let texts: Vec<String> = Category::iter()
                .map(|c| category_label(c).to_string())
                .chain(ArticleType::iter().map(|a| article_type_label(a).to_string()))
                .collect();
            generate_embeddings(&texts).await
        })
        .await?;

    let (cat_embs, type_embs) = embs.split_at(Category::COUNT);

    //  Zip directly with the iterators to find the best match
    let category = Category::iter()
        .zip(cat_embs)
        .max_by(|a, b| {
            cosine_similarity(embedding, a.1).total_cmp(&cosine_similarity(embedding, b.1))
        })
        .map(|(c, _)| c)
        .unwrap();

    let article_type = ArticleType::iter()
        .zip(type_embs)
        .max_by(|a, b| {
            cosine_similarity(embedding, a.1).total_cmp(&cosine_similarity(embedding, b.1))
        })
        .map(|(t, _)| t)
        .unwrap();
    Ok((article_type, category))
}
