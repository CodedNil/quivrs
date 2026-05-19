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

/// Positive and negative labels for each category
const fn category_label(category: Category) -> (&'static str, &'static str) {
    match category {
        Category::Business => (
            "Corporate finance, economic trends, startup growth, market investments, earnings reports.",
            "Personal lifestyle, leisure, hobbies, sports matches, fictional entertainment.",
        ),
        Category::Politics => (
            "Government legislation, election cycles, public policy, geopolitics, foreign affairs.",
            "Technology gadgets, software updates, product reviews, individual scientific discoveries.",
        ),
        Category::Law => (
            "Court litigation, legal systems, criminal justice, lawsuits, civil rights violations.",
            "Entertainment news, celebrity culture, fashion trends, food recipes, travel.",
        ),
        Category::Health => (
            "Medical research, healthcare systems, mental wellness, disease prevention, clinical treatments.",
            "Politics, government spending, corporate business, technology hardware, software.",
        ),
        Category::Entertainment => (
            "Movies, TV shows, video games, gaming consoles, pop culture, celebrity interviews.",
            "Serious news, political policy, academic research, legal litigation, corporate finance.",
        ),
        Category::Culture => (
            "Philosophy, societal history, traditional arts, heritage, museum exhibitions.",
            "Current events, immediate breaking news, product marketing, shopping guides, software.",
        ),
        Category::Lifestyle => (
            "Travel experiences, fashion trends, food recipes, cooking techniques, personal leisure.",
            "Industrial hardware, cybersecurity, political policy, legal cases, medical diagnostics.",
        ),
        Category::Environment => (
            "Climate change, ecological conservation, wildlife protection, environmental sustainability.",
            "Stock market updates, gaming reviews, video streaming, fashion trends, corporate earnings.",
        ),
        Category::Technology => (
            "Artificial intelligence, software programming, cybersecurity, hardware gadgets, telecommunications.",
            "Artistic expression, culinary recipes, sports competitions, political history, societal traditions.",
        ),
        Category::Science => (
            "Space physics, biological research, academic experiments, scientific discovery.",
            "Corporate sales, marketing promotions, opinion editorials, gaming news, lifestyle tips.",
        ),
        Category::Education => (
            "University research, teaching methods, student curriculum, educational policy.",
            "Breaking disaster news, commercial advertisements, video game releases, personal blog posts.",
        ),
        Category::Sports => (
            "Professional athletic teams, tournament matches, league competitions, sporting events.",
            "Corporate mergers, political legislation, software development, medical research, fine arts.",
        ),
    }
}

/// Positive and negative labels for each article type
const fn article_type_label(article_type: ArticleType) -> (&'static str, &'static str) {
    match article_type {
        ArticleType::BreakingNews => (
            "Urgent real-time reporting, immediate emergency updates, disaster coverage.",
            "Long-form analysis, historical perspective, personal opinion, product evaluation.",
        ),
        ArticleType::News => (
            "Factual journalistic reporting, current event coverage, company announcements, industry updates.",
            "Subjective critique, hands-on product testing, marketing sales promotion, personal blog diary.",
        ),
        ArticleType::Opinion => (
            "Editorial commentary, subjective viewpoint, personal perspective, persuasive argument.",
            "Objective reporting, breaking news facts, technical instruction, company press release.",
        ),
        ArticleType::Marketing => (
            "Product recommendations, gift guides, affiliate sales links, promotional deals, advertisement.",
            "Hard news, investigative reporting, academic research, emergency update, neutral observation.",
        ),
        ArticleType::Review => (
            "Critical assessment, hands-on product evaluation, performance rating, service critique.",
            "Press release, news announcement, factual update, corporate overview, non-tested speculation.",
        ),
        ArticleType::Interview => (
            "One-on-one conversation, Q&A transcript, personal exclusive profile, spoken dialogue.",
            "Summary report, third-party observation, marketing copy, tutorial instructions.",
        ),
        ArticleType::Guide => (
            "Technical tutorial, step-by-step instructions, how-to walkthrough, setup explainer.",
            "Subjective opinion, argumentative piece, breaking news event, entertainment review.",
        ),
        ArticleType::Feature => (
            "Long-form investigative research, historical context, in-depth exploration, magazine piece.",
            "Brief update, breaking news headline, sales advertisement, short social media post.",
        ),
        ArticleType::Blog => (
            "Individual perspective, personal developer diary, casual commentary, subjective narrative.",
            "Formal news report, corporate announcement, scientific research, official policy document.",
        ),
        ArticleType::Newsletter => (
            "Direct email subscription, digest mailing list, corporate publication update.",
            "Real-time breaking news, social media status, singular video essay, individual post.",
        ),
        ArticleType::Video => (
            "YouTube visual media, video essay, film documentary, online streaming content.",
            "Text-based news article, formal editorial, technical whitepaper, email newsletter.",
        ),
        ArticleType::Post => (
            "Social media status update, short message, Twitter thread, user-generated micro-blog.",
            "Professional journalism, magazine feature, academic research, corporate whitepaper.",
        ),
    }
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
            let mut all_texts = Vec::with_capacity((Category::COUNT + ArticleType::COUNT) * 2);

            for c in Category::iter() {
                all_texts.push(category_label(c).0.to_string());
            }
            for c in Category::iter() {
                all_texts.push(category_label(c).1.to_string());
            }
            for a in ArticleType::iter() {
                all_texts.push(article_type_label(a).0.to_string());
            }
            for a in ArticleType::iter() {
                all_texts.push(article_type_label(a).1.to_string());
            }

            let mut embs = generate_embeddings(&all_texts).await?;

            let type_neg = embs.split_off(embs.len() - ArticleType::COUNT);
            let type_pos = embs.split_off(embs.len() - ArticleType::COUNT);
            let cat_neg = embs.split_off(embs.len() - Category::COUNT);
            let cat_pos = embs;

            anyhow::Ok(LabelEmbeddings {
                cat_pos,
                cat_neg,
                type_pos,
                type_neg,
            })
        })
        .await?;

    let score = |pos: &[f32], neg: &[f32]| {
        cosine_similarity(embedding, pos) - cosine_similarity(embedding, neg)
    };

    let best_cat = Category::iter()
        .zip(labels.cat_pos.iter().zip(&labels.cat_neg))
        .map(|(variant, (pos, neg))| (variant, score(pos, neg)))
        .max_by(|(_, s1), (_, s2)| s1.total_cmp(s2))
        .map(|(c, _)| c)
        .unwrap();

    let best_type = ArticleType::iter()
        .zip(labels.type_pos.iter().zip(&labels.type_neg))
        .map(|(variant, (pos, neg))| (variant, score(pos, neg)))
        .max_by(|(_, s1), (_, s2)| s1.total_cmp(s2))
        .map(|(t, _)| t)
        .unwrap();

    Ok((best_type, best_cat))
}
