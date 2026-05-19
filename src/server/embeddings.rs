use crate::shared::{ArticleSource, ArticleType, Category};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::LazyLock;
use strum::{EnumCount, IntoEnumIterator};
use tokio::sync::{Mutex, OnceCell};

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const MODEL_NAME: &str = "EmbeddingGemma300M";

const EMBEDDING_TITLE_WEIGHT: f32 = 0.75;

static STATE: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    STATE.lock().await.embed(texts, None)
}

pub async fn generate_article_embeddings(articles: &[ArticleSource]) -> Result<Vec<Vec<f32>>> {
    let texts = articles
        .iter()
        .flat_map(|s| [s.title.clone(), s.summary.clone()])
        .collect::<Vec<_>>();

    Ok(generate_embeddings(&texts)
        .await?
        .chunks_exact(2)
        .map(|pair| {
            let (t, s) = (&pair[0], &pair[1]);
            let mut combined: Vec<f32> = t
                .iter()
                .zip(s)
                .map(|(a, b)| EMBEDDING_TITLE_WEIGHT * a + (1.0 - EMBEDDING_TITLE_WEIGHT) * b)
                .collect();
            let norm: f32 = combined.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut combined {
                    *x /= norm;
                }
            }
            combined
        })
        .collect())
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
            "Business. Corporate finance, economic trends, startup growth, market investments, earnings reports.",
            "Consumer shopping guides, retail sale deals, personal lifestyle, leisure, hobbies, sports matches, fictional entertainment, technology, ai, llm.",
        ),
        Category::Politics => (
            "Politics. Government legislation, election, policy, geopolitics, foreign-affairs, HMRC, refugee.",
            "Technology gadgets, software programming tutorials, developer content, legal regulatory compliance, crime allegations, product reviews, individual scientific discoveries.",
        ),
        Category::Law => (
            "Law. Court litigation, legal systems, criminal justice, regulatory compliance, content regulation, hate crime law, civil rights violations, stolen, thieves, victims, emergency services, police, murder, violence, assault, theft.",
            "Entertainment news, celebrity, fashion trends, food recipes, travel.",
        ),
        Category::Health => (
            "Health. Medical research, healthcare systems, mental wellness, disease prevention, clinical treatments, meningitis, infection spread, virus, bacteria, covid-19, epidemic, pandemic, hospital, ambulance.",
            "Politics, government spending, corporate business, technology hardware, software.",
        ),
        Category::Entertainment => (
            "Entertainment. Movies, TV shows, games, gaming, pop culture, celebrity, netflix, disney, thriller, fantasy.",
            "Serious news, political policy, academic research, legal litigation, corporate finance, physical hardware.",
        ),
        Category::Culture => (
            "Culture. Philosophy, societal history, traditional arts, heritage, museum exhibitions, royal visits.",
            "Current events, immediate breaking news, product marketing, shopping guides, software.",
        ),
        Category::Lifestyle => (
            "Lifestyle. Travel experiences, fashion trends, food recipes, home and garden, outdoor living, personal leisure.",
            "Industrial hardware, cybersecurity, software tools, digital applications, political policy, legal cases, medical diagnostics, celebrity, technology gadgets.",
        ),
        Category::Environment => (
            "Environment. Climate change, ecological conservation, wildlife protection, environmental sustainability.",
            "Stock market updates, gaming reviews, video streaming, fashion trends, corporate earnings.",
        ),
        Category::Technology => (
            "Technology. Artificial intelligence, software programming, cybersecurity, hardware gadgets, creative software tools, digital art applications, 3D modeling, telecommunications, operating system, python, javascript, robot, llm, ai.",
            "Artistic expression, culinary recipes, sports competitions, political history, societal traditions.",
        ),
        Category::Science => (
            "Science. Space physics, biological research, academic experiments, scientific discovery.",
            "Corporate sales, marketing promotions, opinion editorials, gaming news, lifestyle tips, crime.",
        ),
        Category::Education => (
            "Education. University research, teaching methods, student curriculum, educational policy.",
            "Breaking disaster news, commercial advertisements, game, personal blog posts.",
        ),
        Category::Sports => (
            "Sports. Professional athletic teams, tournament matches, league competitions, sporting events.",
            "Corporate mergers, political legislation, software development, medical research, fine arts, thieves.",
        ),
    }
}

/// Positive and negative labels for each article type
const fn article_type_label(article_type: ArticleType) -> (&'static str, &'static str) {
    match article_type {
        ArticleType::BreakingNews => (
            "Live emergency alert, immediate breaking news flash, disaster or crisis happening right now, important vital news.",
            "Video content, watch articles, long-form analysis, historical perspective, personal opinion, product evaluation, humorous, unimportant trivial news.",
        ),
        ArticleType::News => (
            "Neutral factual reporting on current events, political developments, official announcements, crime and court case updates, journalistic coverage with attributed sources.",
            "Subjective critique, hands-on product testing, marketing sales promotion, personal blog diary.",
        ),
        ArticleType::Opinion => (
            "Editorial opinion piece, columnist subjective viewpoint, first-person argument.",
            "Balanced neutral reporting, narrative cultural travel writing, descriptive long-form prose, factual political coverage with multiple perspectives, objective news article, breaking news facts, technical instruction, press release.",
        ),
        ArticleType::Marketing => (
            "Product recommendations, gift guides, affiliate sales links, promotional deals, advertisement, sponsored content.",
            "Hard news, investigative reporting, academic research, emergency update, neutral observation.",
        ),
        ArticleType::Review => (
            "Critical assessment, hands-on product evaluation, performance rating, service critique, new product review.",
            "Press release, news announcement, factual update, corporate overview, non-tested speculation.",
        ),
        ArticleType::Interview => (
            "Journalist questions followed by subject answers in Q&A format, one-on-one exclusive interview transcript.",
            "News report quoting officials, article with attributed third-party statements, marketing copy, tutorial instructions.",
        ),
        ArticleType::Guide => (
            "Technical tutorial, step-by-step instructions, how-to walkthrough, setup explainer.",
            "Subjective opinion, argumentative piece, breaking news event, entertainment review.",
        ),
        ArticleType::Feature => (
            "Long-form magazine journalism, comprehensive investigative deep dive, analytical piece, narrative cultural travel writing, Sunday long read.",
            "Brief factual news report, current-event update, breaking headline, sales advertisement, social media post.",
        ),
        ArticleType::Blog => (
            "Personal amateur blog post, non-professional individual writing, personal website dev-diary.",
            "Formal news report, large corporation, professional journalist, scientific research, official policy document.",
        ),
        ArticleType::Newsletter => (
            "Direct email subscription, digest mailing list, corporate publication update.",
            "Real-time breaking news, social media status, singular video essay, individual post.",
        ),
        ArticleType::Video => (
            "YouTube visual media, video essay, film documentary, online streaming content.",
            "Written text article listing shows or episodes to watch, streaming content guide, formal editorial, technical whitepaper, email newsletter.",
        ),
        ArticleType::Post => (
            "Social media post on Twitter, Facebook status update, Instagram caption, Reddit submission, short user-generated content on a social platform.",
            "News article, journalistic report, official political announcement, magazine feature, academic research, corporate whitepaper.",
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
