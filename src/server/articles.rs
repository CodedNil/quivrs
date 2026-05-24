use crate::{
    server::{
        database,
        embeddings::{
            EMBEDDING_MODEL_NAME, article_text, calculate_preference_score, classify,
            cosine_similarity, generate_article_embeddings,
        },
        llm_functions::run,
        parsers::{feeds::scan_feed, fetch_page_content},
    },
    shared::{Article, ArticleSource, ArticleStatus, PendingSource},
};
use anyhow::{Result, anyhow};
use futures::future::join_all;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
};
use tokio::fs;
use tracing::{error, info, warn};
use uuid::Uuid;

const SIMILARITY_THRESHOLD: f32 = 0.68;
const LIKED_SERVE_THRESHOLD: f64 = 0.8; // If the liked guess is above this, it is automatically served
const CATEGORY_MIN_NEW_ARTICLES: i64 = 4; // Minimum number of new articles to maintain in each category
const TIME_BONUS_MAX: f32 = 0.1;
const TIME_WINDOW_DAYS: i64 = 2;

pub async fn refresh_all_feeds() -> Result<()> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.ron".to_string());
    let config_file: HashMap<String, String> =
        ron::from_str(&fs::read_to_string(&config_path).await?)
            .map_err(|e| anyhow!("Failed to read {config_path}: {e}"))?;

    info!("Scanning {} feeds", config_file.len());
    let candidate_urls: HashSet<String> =
        join_all(config_file.into_iter().map(|(id, url)| async move {
            scan_feed(&url).await.unwrap_or_else(|err| {
                warn!(feed_id = %id, "Feed scan failed: {err:#}");
                vec![]
            })
        }))
        .await
        .into_iter()
        .flatten()
        .collect();

    let new_urls = database::filter_new_urls(&candidate_urls).await?;
    if new_urls.is_empty() {
        info!("No new entries found");
        return Ok(());
    }

    info!("Fetching content for {} new articles...", new_urls.len());
    let results = join_all(
        new_urls
            .iter()
            .map(|url| async { fetch_page_content(url).await }),
    )
    .await;

    let mut new_entries: Vec<PendingSource> = vec![];
    let mut dismissed_urls: Vec<String> = vec![];
    for (url, result) in new_urls.into_iter().zip(results) {
        match result {
            Ok(Some(source)) => new_entries.push(source),
            Ok(None) => dismissed_urls.push(url),
            Err(e) => warn!("Failed to fetch {url}: {e:#}"),
        }
    }
    database::mark_urls_dismissed(&dismissed_urls).await?;

    if new_entries.is_empty() {
        return Ok(());
    }

    info!(
        "Generating embeddings for {} new articles...",
        new_entries.len()
    );
    let texts: Vec<String> = new_entries.iter().map(|a| article_text(a, 5)).collect();
    let embeddings = generate_article_embeddings(&texts)
        .await
        .inspect_err(|e| error!("Batch embedding generation failed: {e}"))?;

    for ((mut source, embedding_text), embedding) in new_entries
        .into_iter()
        .zip(texts.into_iter())
        .zip(embeddings.into_iter())
    {
        let Ok(category) = classify(&embedding).await else {
            warn!("Classification failed for '{}'", source.title);
            continue;
        };

        source.embedding = embedding;
        source.embedding_text = embedding_text;
        source.embedding_model = EMBEDDING_MODEL_NAME.to_string();
        source.category = category;

        info!(
            "[NEW] {}",
            format!("'{}' {}", article_text(&source, 1), category)
        );
        database::insert_source(&source).await?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleEntry {
    pub title: String,
    pub description: String,
    pub content: String,
    pub sidebar: String,
    pub thumbnail: String,
}

/// Scans all pending sources and promotes high-quality or necessary content into full articles.
pub async fn promote_articles() -> Result<()> {
    let pending = database::get_pending_sources().await?;
    if pending.is_empty() {
        return Ok(());
    }

    let rated_embeddings = database::get_rated_article_embeddings().await?;

    // Calculate scores and sort
    let mut scored_pending: Vec<(f64, PendingSource)> = pending
        .into_iter()
        .map(|p| {
            let score = f64::from(calculate_preference_score(&p.embedding, &rated_embeddings));
            (score, p)
        })
        .collect();

    scored_pending.sort_by(|a, b| b.0.total_cmp(&a.0));

    info!(
        "Running promote_articles with {} pending sources",
        scored_pending.len()
    );

    let cat_counts = database::get_category_article_counts().await?;
    let mut promoted_urls = HashSet::new();

    for (i, (estimated_liked, candidate)) in scored_pending.iter().enumerate() {
        if promoted_urls.contains(&candidate.url) {
            continue;
        }

        let count = cat_counts.get(&candidate.category).copied().unwrap_or(0);
        if *estimated_liked < LIKED_SERVE_THRESHOLD && count >= CATEGORY_MIN_NEW_ARTICLES {
            continue;
        }

        info!(
            "[PROMOTING] '{}' (score: {:.2}, cat: {}, current_count: {})",
            candidate.title, estimated_liked, candidate.category, count
        );

        let mut sources_to_merge = vec![candidate.clone()];
        promoted_urls.insert(candidate.url.clone());

        let candidate_ts = candidate.published.timestamp() as f32;
        let half_life_secs = 4.0 * 3600.0_f32;

        for other_pair in scored_pending.iter().skip(i + 1) {
            let other = &other_pair.1;
            if other.category != candidate.category || promoted_urls.contains(&other.url) {
                continue;
            }

            if (candidate.published - other.published).num_days().abs() > TIME_WINDOW_DAYS {
                continue;
            }

            let diff_secs = (candidate_ts - other.published.timestamp() as f32).abs();
            let time_bonus =
                TIME_BONUS_MAX * (-std::f32::consts::LN_2 * diff_secs / half_life_secs).exp();

            let sim = cosine_similarity(&candidate.embedding, &other.embedding);
            let score = sim + time_bonus;

            if score >= SIMILARITY_THRESHOLD {
                info!(
                    "  [MERGE] adding '{}' (sim: {:.2}, bonus: {:.2}, total: {:.2})",
                    other.title, sim, time_bonus, score
                );
                sources_to_merge.push(other.clone());
                promoted_urls.insert(other.url.clone());
            }
        }

        let article_sources = sources_to_merge
            .iter()
            .map(|ps| ArticleSource {
                url: ps.url.clone(),
                domain: ps.domain.clone(),
            })
            .collect();

        match generate_promoted_content(sources_to_merge.clone()).await {
            Ok(entry) => {
                let article = Article {
                    id: Uuid::new_v4(),
                    sources: article_sources,
                    title: entry.title,
                    description: entry.description,
                    content: entry.content,
                    sidebar: entry.sidebar,
                    thumbnail: entry.thumbnail,
                    published: candidate.published,
                    category: candidate.category,
                    status: ArticleStatus::New,
                    binned_at: None,
                    rating: None,
                    embedding: candidate.embedding.clone(),
                    embedding_text: candidate.embedding_text.clone(),
                    embedding_model: candidate.embedding_model.clone(),
                };
                let urls: Vec<String> = sources_to_merge.into_iter().map(|s| s.url).collect();
                database::insert_promoted_article(&article, &urls).await?;
            }
            Err(e) => error!(
                "Failed to generate content for '{}': {}",
                candidate.title, e
            ),
        }
    }

    Ok(())
}

/// Synthesizes multiple article sources into a single, high-quality long-form article using an LLM.
async fn generate_promoted_content(sources: Vec<PendingSource>) -> Result<ArticleEntry> {
    let images = sources
        .iter()
        .flat_map(|source| &source.images)
        .map(|(src, caption)| {
            if caption.is_empty() {
                format!("[{src}]")
            } else {
                format!("[{src}, {caption}]")
            }
        })
        .join(" ");

    let articles_content = sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            format!(
                "Source {} - Title: {} - URL: {} - Content: {}",
                i + 1,
                source.title,
                source.url,
                source.content.replace('\n', " ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let context = format!(
        r#"You are a professional digital editor journalist. Synthesize the provided sources into a high-quality, long-form digital article.

### OUTPUT FORMAT
You MUST return a JSON object with this exact structure:
{{
  "title": "Articles title, kept concise and descriptive, max 8 words",
  "description": "Short informative summary, a few sentences max and no newlines",
  "thumbnail": "URL for the thumbnail image, landscapes/architectural/unpopulated/scenic/low complexity image preferred",
  "content": "HTML string for the main article body",
  "sidebar": "HTML string for the sidebar, key facts, summaries, or metadata, written in HTML"
}}

### CONTENT COMPONENT LIBRARY
Use these HTML patterns to structure the "content" and "sidebar" fields:

1. **Standard Text**: Use <h2> through <h6> for sections. Use <p> for body text.
2. **Feature Box**: <div class="box"><strong>Title</strong><p>Description</p></div>
3. **Layout Grids**:
   - Vertical Stack: <div class="flexbox-columns">...</div>
   - Horizontal Grid: <div class="flexbox-rows">...</div> (Great for image galleries or side-by-side boxes)
4. **Visuals**: <figure><img src="..." alt="..." /><figcaption>Caption</figcaption></figure>
5. **Data Table**: <table class="info-table"><tr><th>Key</th><td>Value</td></tr></table>
6. **Timeline**: <div class="timeline"><div class="timeline-item"><span class="date">YYYY</span><span class="event">Event</span></div></div>
7. **Quotes**: <blockquote class="quote">Expert statement...</blockquote>

### EDITORIAL REQUIREMENTS
- **Image Usage**: Use most images provided, have a hero image near the start, distribute the rest logically. Ignore images that are clearly unnecessary, like branding.
- **Redundancy**: Don't repeat content on both the main body and the sidebar. Aim for concise informative prose.

- **Content: Main Body**:
  - Start with a 2-3 paragraph overview.
  - Increase depth and detail as the article progresses.
  - **Timeline (Optional)**: Include a "Timeline" section if the subject has a clear chronological history.
  - **Perspectives (Optional)**: Include a "Perspectives" section at the end using <div class="flexbox-rows"> with <div class="box"> elements to show different viewpoints (e.g., "Critics", "Supporters", "Experts").
- **Tone**: Professional, objective, and deeply informative.

- **Sidebar: Key Details**:
  - **Maps/Infographics**: If a map or small infographic is available, place it here.
  - **Metadata Table**: Use an `info-table` for key data points. Examples:
    - News/Events: "Date", "Location", "Key Figures".
    - Reviews: "Price", "Rating", "Pros", "Cons".
    - Entities: "Founded", "Headquarters", "Key People".
  - **Quick Highlights**: A <ul> of key takeaways or highlights.

### DATA INPUTS
Available Images:
{images}

Sources with information:
{articles_content}"#
    );

    let entry = run::<ArticleEntry>(&context)
        .await
        .map_err(|e| anyhow!("Article generation failed: {e}"))?;

    // Clean the content using ammonia to prevent XSS attacks
    let content_sanitized = ammonia::Builder::default()
        .add_generic_attributes(&["class"])
        .clean(&entry.content)
        .to_string();
    let sidebar_sanitized = ammonia::Builder::default()
        .add_generic_attributes(&["class"])
        .clean(&entry.sidebar)
        .to_string();

    Ok(ArticleEntry {
        content: content_sanitized,
        sidebar: sidebar_sanitized,
        ..entry
    })
}
