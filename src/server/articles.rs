use crate::{
    server::{
        database,
        embeddings::{
            EMBEDDING_TITLE_REPEAT, article_text, classify, embedding_model_id, generate_embeddings,
        },
        llm_functions::run,
        parsers::{
            feeds::scan_feed,
            fetch_page_content, fetch_page_content_with_hint,
            web_search::{SearchResult, search_article_urls},
        },
    },
    shared::{Article, ArticleSource, ArticleStatus, PendingSource, Rating},
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use futures::future::join_all;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    env,
    f32::consts::LN_2,
};
use tokio::{fs, time::Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

const LIKED_SERVE_THRESHOLD: f32 = 0.9; // If the liked guess is above this, it is automatically served
const LIKED_MIN_THRESHOLD: f32 = 0.6; // If the liked guess is above this, it is served to meet quota

const MERGE_SIMILARITY_THRESHOLD: f32 = 0.58; // Threshold for merging similar articles
const TIME_BONUS_MAX: f32 = 0.03; // How much to boost the score for merging a recent article

const SENTIMENT_BONUS: f32 = 0.1; // How much to boost the score for a positive article
const IMPORTANCE_BONUS: f32 = 0.1; // How much to boost the score for an important article

struct ScoredPendingSource {
    source: PendingSource,
    estimated_liked: f32,
    bonus: f32,
    category_target: i64,
}

/// Minimum number of new articles to maintain in each category
pub const fn category_new_articles(rating: Rating) -> i64 {
    match rating {
        Rating::Hated => 0,
        Rating::Disliked => 1,
        Rating::Neutral => 2,
        Rating::Liked => 4,
        Rating::Loved => 6,
    }
}

/// Max amount to boost or dampen the score for a rated category
pub const fn category_bonus(rating: Rating) -> f32 {
    match rating {
        Rating::Hated => -0.2,
        Rating::Disliked => -0.1,
        Rating::Neutral => 0.0,
        Rating::Liked => 0.1,
        Rating::Loved => 0.3,
    }
}

/// Max amount to boost or dampen the score for a rated region.
pub const fn region_bonus(rating: Rating) -> f32 {
    match rating {
        Rating::Hated => -0.2,
        Rating::Disliked => -0.1,
        Rating::Neutral => 0.0,
        Rating::Liked => 0.1,
        Rating::Loved => 0.3,
    }
}

/// Max amount to boost or dampen the score for a rated domain
pub const fn domain_bonus(rating: Rating) -> f32 {
    match rating {
        Rating::Hated => -0.4,
        Rating::Disliked => -0.2,
        Rating::Neutral => 0.0,
        Rating::Liked => 0.15,
        Rating::Loved => 0.4,
    }
}

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
    let texts: Vec<String> = new_entries
        .iter()
        .map(|a| article_text(a, EMBEDDING_TITLE_REPEAT))
        .collect();
    let embeddings = generate_embeddings(&texts)
        .await
        .inspect_err(|e| error!("Batch embedding generation failed: {e}"))?;

    let mut classified_sources = Vec::with_capacity(new_entries.len());
    for ((mut source, embedding_text), embedding) in
        new_entries.into_iter().zip(texts).zip(embeddings)
    {
        let (category, region, sentiment, importance) = match classify(&embedding).await {
            Ok(scores) => scores,
            Err(err) => {
                warn!("Classification failed for '{}': {err:#}", source.title);
                continue;
            }
        };

        source.embedding = embedding;
        source.embedding_text = embedding_text;
        source.embedding_model = embedding_model_id();
        source.category = category;
        source.region = region;
        source.sentiment = sentiment;
        source.importance = importance;

        info!(
            "[NEW] {}",
            format!(
                "'{}' {}, {}, sentiment: {:.2} importance: {:.2}",
                article_text(&source, 1),
                category,
                region,
                sentiment,
                importance
            )
        );
        classified_sources.push(source);
    }
    database::insert_sources(classified_sources).await?;

    Ok(())
}

#[derive(Deserialize)]
pub struct ArticleEntry {
    pub title: String,
    pub description: String,
    pub content: String,
    pub sidebar: String,
    pub thumbnail: String,
}

/// Scans all pending sources and promotes the best content into full articles.
pub async fn promote_articles() -> Result<()> {
    let pending = database::get_pending_sources().await?;
    if pending.is_empty() {
        return Ok(());
    }

    let item_ratings = database::get_all_item_ratings().await?;

    // Calculate scores and sort
    let mut scored_pending = Vec::with_capacity(pending.len());
    let start = std::time::Instant::now();
    for p in pending {
        let estimated_liked = database::get_preference_score(&p.embedding).await?;
        let mut bonus = 0.0;

        // Boost for sentiment and importance
        bonus += p.sentiment * SENTIMENT_BONUS;
        bonus += p.importance * IMPORTANCE_BONUS;

        // Boost for category, region, and domain ratings.
        let category_target = item_ratings
            .get(&format!("category:{}", p.category))
            .map_or(category_new_articles(Rating::Neutral), |rating| {
                bonus += category_bonus(*rating);
                category_new_articles(*rating)
            });
        if let Some(rating) = item_ratings.get(&format!("region:{}", p.region)) {
            bonus += region_bonus(*rating);
        }
        if let Some(rating) = item_ratings.get(&format!("domain:{}", p.domain)) {
            bonus += domain_bonus(*rating);
        }

        scored_pending.push(ScoredPendingSource {
            source: p,
            estimated_liked,
            bonus,
            category_target,
        });
    }
    info!(
        "Scored {} pending sources in {:?}",
        scored_pending.len(),
        start.elapsed()
    );

    scored_pending.sort_by(|a, b| b.estimated_liked.total_cmp(&a.estimated_liked));

    info!(
        "Running promote_articles with {} pending sources",
        scored_pending.len()
    );
    let start = Instant::now();

    let mut cat_counts = database::get_category_article_counts().await?;
    let mut promoted_urls = HashSet::new();

    for scored in scored_pending {
        let ScoredPendingSource {
            source: candidate,
            estimated_liked,
            bonus,
            category_target,
        } = scored;
        if promoted_urls.contains(&candidate.url) {
            continue;
        }

        let count = cat_counts.get(&candidate.category).copied().unwrap_or(0);
        let score = estimated_liked + bonus;

        let should_keep = score >= LIKED_SERVE_THRESHOLD
            || (score >= LIKED_MIN_THRESHOLD && count < category_target);
        if !should_keep {
            continue;
        }

        info!(
            "[PROMOTING] '{} {}' (score: {:.2} ({:.2} bonus), cat: {}, reg: {}, sentiment: {:.2}, importance: {:.2})",
            candidate.url,
            candidate.title,
            estimated_liked,
            bonus,
            candidate.category,
            candidate.region,
            candidate.sentiment,
            candidate.importance
        );

        promoted_urls.insert(candidate.url.clone());
        let similar_sources = database::get_similar_pending_sources(&candidate.embedding).await?;

        let candidate_ts = candidate.published.timestamp() as f32;
        let half_life_secs = 4.0 * 3600.0;

        let mut sources_to_merge = vec![candidate];

        for (other, similarity) in similar_sources {
            let diff_secs = (candidate_ts - other.published.timestamp() as f32).abs();
            let time_bonus = TIME_BONUS_MAX * (-LN_2 * diff_secs / half_life_secs).exp();

            let score = similarity + time_bonus;
            if score >= MERGE_SIMILARITY_THRESHOLD {
                if promoted_urls.insert(other.url.clone()) {
                    info!(
                        "  [MERGE] adding '{}' (sim: {:.2}, bonus: {:.2})",
                        other.title, similarity, time_bonus
                    );
                    sources_to_merge.push(other);
                }
            } else if score >= MERGE_SIMILARITY_THRESHOLD - 0.1 {
                info!(
                    "  [MISSED] near miss '{}' (sim: {:.2}, bonus: {:.2})",
                    other.title, similarity, time_bonus
                );
            }
        }

        match fetch_web_search_merge_sources(&sources_to_merge).await {
            Ok(search_sources) => {
                for source in search_sources {
                    if promoted_urls.insert(source.url.clone()) {
                        info!("  [WEB MERGE] adding '{}'", source.title);
                        sources_to_merge.push(source);
                    }
                }
            }
            Err(err) => warn!(
                "Web search merge enrichment failed for '{}': {err:#}",
                sources_to_merge[0].title
            ),
        }

        match generate_promoted_content(&sources_to_merge).await {
            Ok(entry) => {
                let mut sources = sources_to_merge.into_iter();
                let candidate = sources.next().expect("candidate source is present");
                let category = candidate.category;
                let mut urls = vec![candidate.url.clone()];
                let mut article_sources = vec![ArticleSource {
                    url: candidate.url,
                    domain: candidate.domain,
                }];
                for source in sources {
                    urls.push(source.url.clone());
                    article_sources.push(ArticleSource {
                        url: source.url,
                        domain: source.domain,
                    });
                }

                let article = Article {
                    id: Uuid::new_v4(),
                    sources: article_sources,
                    title: entry.title,
                    description: entry.description,
                    content: entry.content,
                    sidebar: entry.sidebar,
                    thumbnail: entry.thumbnail,
                    published: candidate.published,
                    category,
                    region: candidate.region,
                    status: ArticleStatus::New,
                    status_changed: Utc::now(),
                    rating: None,
                    estimated_liked: Some(estimated_liked),
                    embedding: candidate.embedding,
                    embedding_text: candidate.embedding_text,
                    embedding_model: candidate.embedding_model,
                    sentiment: candidate.sentiment,
                    importance: candidate.importance,
                };
                database::insert_promoted_article(article, urls).await?;
                *cat_counts.entry(category).or_default() += 1;
            }
            Err(e) => error!(
                "Failed to generate content for '{}': {}",
                sources_to_merge[0].title, e
            ),
        }
    }

    info!("Promoting articles finished in {:?}", start.elapsed());

    Ok(())
}

async fn fetch_web_search_merge_sources(
    initial_sources: &[PendingSource],
) -> Result<Vec<PendingSource>> {
    let mut searched_urls: HashMap<String, Option<chrono::DateTime<Utc>>> = HashMap::new();
    for result in join_all(
        initial_sources
            .iter()
            .map(|source| search_article_urls(&source.title)),
    )
    .await
    {
        match result {
            Ok(urls) => {
                for SearchResult { url, published } in urls {
                    searched_urls
                        .entry(url)
                        .and_modify(|existing| {
                            if existing.is_none() {
                                *existing = published;
                            }
                        })
                        .or_insert(published);
                }
            }
            Err(err) => warn!("Web search failed: {err:#}"),
        }
    }

    let new_urls = database::filter_new_urls(&searched_urls.keys().cloned().collect()).await?;
    info!(
        "Fetching {} web search results for merge enrichment",
        new_urls.len()
    );

    let mut fetched_sources = Vec::new();
    let mut processed_urls = Vec::new();
    for (url, result) in new_urls.iter().zip(
        join_all(new_urls.iter().map(|url| async {
            fetch_page_content_with_hint(url, searched_urls.get(url).copied().flatten()).await
        }))
        .await,
    ) {
        match result {
            Ok(Some(source)) => fetched_sources.push(source),
            Ok(None) => processed_urls.push(url.clone()),
            Err(err) => warn!("Failed to fetch web search result {url}: {err:#}"),
        }
    }

    let texts = fetched_sources
        .iter()
        .map(|source| article_text(source, EMBEDDING_TITLE_REPEAT))
        .collect::<Vec<_>>();
    let embeddings = generate_embeddings(&texts)
        .await
        .inspect_err(|err| error!("Web search embedding generation failed: {err}"))?;

    let mut merged_sources = Vec::new();
    let half_life_secs = 4.0 * 3600.0;
    for ((mut source, embedding_text), embedding) in
        fetched_sources.into_iter().zip(texts).zip(embeddings)
    {
        let best_score = initial_sources
            .iter()
            .map(|initial| {
                let diff_secs =
                    (initial.published.timestamp() - source.published.timestamp()).abs() as f32;
                let time_bonus = TIME_BONUS_MAX * (-LN_2 * diff_secs / half_life_secs).exp();
                cosine_similarity(&initial.embedding, &embedding) + time_bonus
            })
            .fold(f32::NEG_INFINITY, f32::max);

        processed_urls.push(source.url.clone());
        if best_score >= MERGE_SIMILARITY_THRESHOLD {
            source.embedding = embedding;
            source.embedding_text = embedding_text;
            source.embedding_model = embedding_model_id();
            merged_sources.push(source);
        } else if best_score >= MERGE_SIMILARITY_THRESHOLD - 0.1 {
            info!(
                "  [WEB MISSED] near miss '{}' (sim: {:.2})",
                source.title, best_score
            );
        }
    }

    database::mark_urls_dismissed(&processed_urls).await?;

    Ok(merged_sources)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Synthesizes multiple article sources into a single, high-quality long-form article using an LLM.
async fn generate_promoted_content(sources: &[PendingSource]) -> Result<ArticleEntry> {
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
  "title": "Articles title, kept concise and descriptive, ideal 8 words, max 12 words, catchy and engaging but not clickbait.",
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
  - **Information**: Include ONLY what is in the source articles, with only minor wording changes and no editorial interpretation. Feel free to move sentences around for better flow when you are combining articles. Only include the same information once, even if it was repeated across multiple sources. You condense into a well crafted and professional overview.

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
    let mut sanitizer = ammonia::Builder::default();
    sanitizer.add_generic_attributes(&["class"]);
    let content_sanitized = sanitizer.clean(&entry.content).to_string();
    let sidebar_sanitized = sanitizer.clean(&entry.sidebar).to_string();

    Ok(ArticleEntry {
        content: content_sanitized,
        sidebar: sidebar_sanitized,
        ..entry
    })
}
