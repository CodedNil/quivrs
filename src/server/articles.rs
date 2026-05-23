use crate::{
    server::{
        database,
        embeddings::{article_text, classify, cosine_similarity, generate_article_embeddings},
        llm_functions::run,
        parsers::{feeds::scan_feed, fetch_page_content},
    },
    shared::{ArticleEntry, ArticleSource},
};
use anyhow::{Result, anyhow};
use chrono::TimeDelta;
use futures::{StreamExt, future::join_all, stream};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    env,
};
use tokio::fs;
use tracing::{error, info, warn};

const SIMILARITY_THRESHOLD: f32 = 0.68;
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

    let mut new_entries: Vec<ArticleSource> = vec![];
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
    let embeddings = generate_article_embeddings(&new_entries)
        .await
        .inspect_err(|e| error!("Batch embedding generation failed: {e}"))?;

    for (source, embedding) in new_entries.into_iter().zip(embeddings.into_iter()) {
        if embedding.is_empty() {
            warn!("Skipping article due to empty embedding: {}", source.title);
            continue;
        }

        let Ok(category) = classify(&embedding).await else {
            warn!("Classification failed for '{}'", source.title);
            continue;
        };
        // Find the highest similarity match
        let candidates = database::get_embedding_candidates(
            (source.published - TimeDelta::days(TIME_WINDOW_DAYS)).timestamp(),
            (source.published + TimeDelta::days(TIME_WINDOW_DAYS)).timestamp(),
        )
        .await?;

        let source_ts = source.published.timestamp() as f32;
        let half_life_secs = 4.0 * 3600.0_f32;

        let mut best: Option<(uuid::Uuid, &str, f32)> = None;
        let mut highest: Option<(&str, f32)> = None;
        for c in candidates.iter().filter(|c| !c.embedding.is_empty()) {
            let diff_secs = (source_ts - c.published as f32).abs();
            let time_bonus =
                TIME_BONUS_MAX * (-std::f32::consts::LN_2 * diff_secs / half_life_secs).exp();
            let score = cosine_similarity(&embedding, &c.embedding) + time_bonus;
            if highest.is_none_or(|(_, s)| score > s) {
                highest = Some((&c.title, score));
            }
            if score >= SIMILARITY_THRESHOLD
                && c.category == category
                && best.is_none_or(|(_, _, s)| score > s)
            {
                best = Some((c.id, &c.title, score));
            }
        }

        let article_overview = format!("'{}' {}", article_text(&source, 1), category);
        if let Some((article_id, existing_title, score)) = best {
            info!(
                "[MERGE] {} → '{}' (score {score:.2})",
                article_overview, existing_title
            );
            database::merge_into_article(article_id, &source).await?;
        } else {
            if let Some((closest_title, sim)) = highest {
                info!(
                    "[NEW] {} - Highest {sim:.2} '{}'",
                    article_overview, closest_title
                );
            } else {
                info!("[NEW] {}", article_overview);
            }
            database::insert_article(&source, &embedding, category).await?;
        }
    }

    Ok(())
}

pub async fn regenerate_articles() -> Result<()> {
    if env::var("OPENROUTER").is_err() {
        warn!("OPENROUTER environment variable not set. Skipping generation.");
        return Ok(());
    }

    let targets = database::get_regeneration_targets().await?;
    if targets.is_empty() {
        return Ok(());
    }

    info!("Generating content for {} articles...", targets.len());

    let mut article_stream = stream::iter(targets)
        .map(|(id, sources)| async move { (id, generate_article_content(sources).await) })
        .buffer_unordered(6);

    while let Some((id, result)) = article_stream.next().await {
        match result {
            Ok(entry) => {
                if let Err(err) = database::save_article_entry(id, &entry).await {
                    warn!(article_id = %id, "Failed to save article to database: {err:#}");
                }
            }
            Err(err) => {
                warn!(article_id = %id, "Generation failed: {err:#}");
            }
        }
    }

    Ok(())
}

async fn generate_article_content(sources: Vec<ArticleSource>) -> Result<ArticleEntry> {
    let images = sources
        .iter()
        .flat_map(|source| &source.images)
        .map(|img_str| {
            let (src, caption) = img_str.split_once('|').unwrap_or((img_str, ""));
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
    let sanitizer = ammonia::Builder::default()
        .add_generic_attributes(&["class"])
        .clean(&entry.content)
        .to_string();

    let sidebar_sanitized = entry.sidebar.as_ref().map(|s| {
        ammonia::Builder::default()
            .add_generic_attributes(&["class"])
            .clean(s)
            .to_string()
    });

    Ok(ArticleEntry {
        content: sanitizer,
        sidebar: sidebar_sanitized,
        ..entry
    })
}
