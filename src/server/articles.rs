use crate::{
    server::{
        database,
        embeddings::{article_text, classify, cosine_similarity, generate_article_embeddings},
        llm_functions::run,
        parsers::{feeds::scan_feed, social::fetch_social_content, websites::fetch_source_content},
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
    let results = join_all(new_urls.iter().map(|url| async {
        if url.contains("twitter.com") || url.contains("x.com") || url.contains("bsky.app") {
            fetch_social_content(url).await
        } else {
            fetch_source_content(url.clone()).await
        }
    }))
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
                && c.category == category.to_string()
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
        .buffer_unordered(3);

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
    let articles_content = sources
        .into_iter()
        .enumerate()
        .map(|(i, source)| {
            let images: String = source
                .images
                .iter()
                .map(|img_str| {
                    let (src, caption) = img_str.split_once('|').unwrap_or((img_str, ""));
                    if caption.is_empty() {
                        format!("[{src}]")
                    } else {
                        format!("[{src}, {caption}]")
                    }
                })
                .join(" ");

            format!(
                "Source {} - Title: {} - URL: {} - Images: {} - Content: {}",
                i + 1,
                source.title,
                source.url,
                images,
                source.content.replace('\n', " ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let context = format!(
        r#"Synthesise all sources into a single cohesive article.
Use a few web searches gather the latest information and deep dive of the subject.
EVERYTHING must be entirely factual and based on the sources provided.

Output the result in the following JSON schema:
{{
  "title": Concise and descriptive title, max 8 words
  "description": Short informative summary, a few sentences max, no newlines
  "thumbnail": URL for an image to represent the thumbnail, ideally without people and complexity, needs to look good with text over it
  "popout_image": URL for an image to represent the popout image, ideally the key subject of the article, might be displayed over the thumbnail
  "content": HTML article content
  "sidebar": HTML sidebar content
}}

### Main Content Guidelines ("content" field)
The content should be written in high-quality HTML. Structure it like a professional digital publication with clear sections using headings (h2 to h6) and paragraphs. Do NOT include a main <h1> title.
Many paragraphs are recommended when there is a lot of content, cover the article in depth without repeating yourself.

#### Available Components:
- **Box Component**: A <div> with class 'box'. MUST have a <strong> title on its own row, followed by a <p> for the text.
- **Layout Containers**: Use 'flexbox-columns' for vertical stacks of boxes, or 'flexbox-rows' for horizontal grids of boxes (div with class 'box').
- **Timeline**: A <div> with class 'timeline' containing <div>s with class 'timeline-item'. Each item has a <span class='date'> and a <span class='event'>.
- **Quotes**: A <blockquote> with class 'quote'.

#### Content Structure:
1. **Overview**: Start with a good overview in the first few paragraphs.
2. **Deep Dive**: Increase depth as you progress, including links to sources and embedded media (videos/iframes) where relevant.
3. **Timeline (Optional)**: Include where chronologically relevant. Use the heading "Timeline".
4. **Perspectives (Optional)**: Use the heading "Perspectives". Use 'flexbox-rows' component with up to 4 boxes near the end to show different viewpoints. The box **title** should be the name of the person, organization, or the specific viewpoint (e.g., "Skeptics", "Industry Experts", "The CEO of X").

Images:
Include every available image, don't leave any provided image unused. Place the hero image near the top, and the rest of the images throughout the article as appropriate. Favour high resolution if there are multiple of the same image provided. Wrap in <figure> with an <img> (including `alt`) and a <figcaption>.
You can include multiple images side by side, in a grid etc where appropriate.

### Sidebar Guidelines ("sidebar" field)
The sidebar should be a concise wiki-style summary that adapts to the article type.
- **Headings**: Use a h1 to h6 to separate sections.
- **Metadata Table**: Use a <table> with class 'info-table' for key data points.
    - News: "Date", "Location", "Key Figures".
    - Reviews: "Specs", "Price", "Rating".
    - Entities: "Founded", "Headquarters", "Key People".
- **Highlights (Optional)**: A "Quick Highlights" section using a simple <ul> of key takeaways.

Sources:
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
