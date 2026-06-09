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
            web_search::{SearchResult, image_dedupe_key, search_article_urls, search_image_urls},
        },
    },
    shared::{
        Article, ArticleSource, ArticleStatus, CaptionedImage, PendingSource, Rating,
        theme::THEME_CSS_VARS,
    },
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use futures::future::join_all;
use itertools::Itertools;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    env,
    f32::consts::LN_2,
    fmt::Write,
    sync::LazyLock,
};
use tokio::{fs, time::Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

const LIKED_SERVE_THRESHOLD: f32 = 0.9; // If the liked guess is above this, it is automatically served
const LIKED_MIN_THRESHOLD: f32 = 0.6; // If the liked guess is above this, it is served to meet quota

const MERGE_SIMILARITY_THRESHOLD: f32 = 0.58; // Threshold for merging similar articles
const TIME_BONUS_MAX: f32 = 0.03; // How much to boost the score for merging a recent article
const MERGE_HALF_LIFE_SECS: f32 = 4.0 * 3600.0;

const SENTIMENT_BONUS: f32 = 0.1; // How much to boost the score for a positive article
const IMPORTANCE_BONUS: f32 = 0.1; // How much to boost the score for an important article

static IMG_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<img>(.*?)(?:</img>|<\\img>)").unwrap());
static INTERACTIVE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<interactive(?:\s[^>]*)?>(.*?)</interactive>").unwrap());

const ARTICLE_IFRAME_RESET_CSS: &str = r"
*, *::before, *::after { box-sizing: border-box; }
html {
    min-height: 100%;
    background: transparent;
}
body {
    min-height: 100%;
    margin: 0;
    padding: 1rem;
    overflow: auto;
    background: transparent;
    color: var(--text);
    font-family: Inter, system-ui, sans-serif;
}
";

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
    let config_file: Vec<String> = ron::from_str(&fs::read_to_string(&config_path).await?)
        .map_err(|e| anyhow!("Failed to read {config_path}: {e}"))?;

    info!("Scanning {} feeds", config_file.len());
    let candidate_urls: HashSet<String> = join_all(config_file.into_iter().map(|url| async move {
        scan_feed(&url).await.unwrap_or_else(|err| {
            warn!(url = %url, "Feed scan failed: {err:#}");
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
    let results = join_all(new_urls.iter().map(|url| fetch_page_content(url))).await;

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
            "[NEW] '{}' {}, {}, sentiment: {:.2} importance: {:.2}",
            article_text(&source, 1),
            category,
            region,
            sentiment,
            importance
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
    pub background: String,
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
        let category_rating = item_ratings
            .get(&format!("category:{}", p.category))
            .copied()
            .unwrap_or(Rating::Neutral);
        bonus += category_bonus(category_rating);
        let category_target = category_new_articles(category_rating);
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

        let mut sources_to_merge = vec![candidate];

        for (other, similarity) in similar_sources {
            let diff_secs = (candidate_ts - other.published.timestamp() as f32).abs();
            let time_bonus = TIME_BONUS_MAX * (-LN_2 * diff_secs / MERGE_HALF_LIFE_SECS).exp();

            let score = similarity + time_bonus;
            if score >= MERGE_SIMILARITY_THRESHOLD {
                if !promoted_urls.insert(other.url.clone()) {
                    continue;
                }
                info!(
                    "  [MERGE] adding '{}' (sim: {:.2}, bonus: {:.2})",
                    other.title, similarity, time_bonus
                );
                sources_to_merge.push(other);
            } else if score >= MERGE_SIMILARITY_THRESHOLD - 0.1 {
                info!(
                    "  [MISSED] near miss '{}' (sim: {:.2}, bonus: {:.2})",
                    other.title, similarity, time_bonus
                );
            }
        }

        match fetch_web_search_merge_sources(&sources_to_merge).await {
            Ok(search_sources) => {
                for source in search_sources.into_iter().take(8) {
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
                let (urls, article_sources): (Vec<_>, Vec<_>) = sources_to_merge
                    .iter()
                    .map(|source| {
                        (
                            source.url.clone(),
                            ArticleSource {
                                url: source.url.clone(),
                                domain: source.domain.clone(),
                            },
                        )
                    })
                    .unzip();
                let candidate = sources_to_merge
                    .into_iter()
                    .next()
                    .expect("candidate source is present");
                let category = candidate.category;
                let article = Article {
                    id: Uuid::new_v4(),
                    sources: article_sources,
                    title: entry.title,
                    description: entry.description,
                    content: entry.content,
                    sidebar: entry.sidebar,
                    thumbnail: entry.thumbnail,
                    background: entry.background,
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

    let mut fetched_sources = Vec::new();
    let mut processed_urls = Vec::new();
    for (url, result) in new_urls.iter().zip(
        join_all(new_urls.iter().map(|url| {
            fetch_page_content_with_hint(url, searched_urls.get(url).copied().flatten())
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
    for ((mut source, embedding_text), embedding) in
        fetched_sources.into_iter().zip(texts).zip(embeddings)
    {
        let best_score = initial_sources
            .iter()
            .map(|initial| {
                let diff_secs =
                    (initial.published.timestamp() - source.published.timestamp()).abs() as f32;
                let time_bonus = TIME_BONUS_MAX * (-LN_2 * diff_secs / MERGE_HALF_LIFE_SECS).exp();
                initial
                    .embedding
                    .iter()
                    .zip(&embedding)
                    .map(|(x, y)| x * y)
                    .sum::<f32>()
                    + time_bonus
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

/// Synthesizes multiple article sources into a single, high-quality long-form article using an LLM.
async fn generate_promoted_content(sources: &[PendingSource]) -> Result<ArticleEntry> {
    let mut seen_images = HashSet::new();
    let available_images = sources
        .iter()
        .flat_map(|source| &source.images)
        .filter(|image| seen_images.insert(image.url.clone()))
        .cloned()
        .collect::<Vec<_>>();
    let images = available_images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            let caption = image.caption.trim();
            if caption.is_empty() {
                format!("[{index}][{}]", image.url)
            } else {
                format!("[{index}]({caption})")
            }
        })
        .join("\n");
    let mut seen = HashSet::new();
    let mut suggested_images = Vec::new();
    let results = join_all(
        sources
            .iter()
            .map(|source| search_image_urls(&source.title, 5)),
    )
    .await;
    for result in results {
        match result {
            Ok(results) => {
                for image in results {
                    if seen.insert(image_dedupe_key(&image.url)) {
                        suggested_images.push(CaptionedImage {
                            url: image.url,
                            caption: image.caption,
                        });
                    }
                }
            }
            Err(err) => warn!("Image search failed: {err:#}"),
        }
    }
    let suggested_image_list = suggested_images
        .iter()
        .enumerate()
        .map(|(index, image)| format!("[${index}]({})", image.caption.trim()))
        .join("\n");

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
        .join("\n");

    let context = format!(
        r#"You are a professional digital journalist. Synthesize the provided sources into a high-quality, long-form digital article.

### OUTPUT FORMAT
You MUST return a JSON object with this exact structure:
{{
  "title": "Articles title, kept concise and descriptive, ideal 8 words, max 12 words, catchy and engaging but not clickbait.",
  "description": "Short informative summary, a 24 words max and no newlines",
  "thumbnail": "Image reference such as #0, $0, or search(short visual search term).",
  "background": "Image reference such as #0, $0, or search(short visual search term). Background image for the page, used with heavy blur behind the content. Prefer landscapes, architecture, unpopulated scenes, or low-complexity images when available.",
  "content": "HTML string for the main article body",
  "sidebar": "HTML string for the sidebar, key facts, summaries, or metadata, written in HTML"
}}

### CONTENT COMPONENT LIBRARY
Use these HTML patterns to structure the "content" and "sidebar" fields:

1. **Standard Text**: Use <h2> through <h6> for sections. Use <p> for body text.
2. **Feature Box**: <div class="box"><strong>Title</strong><p>Description</p></div>, renders with a filled background behind it
3. **Layout Grids**:
   - Vertical Stack: <div class="flexbox-columns">...</div>
   - Horizontal Grid: <div class="flexbox-rows">...</div> (Great for image galleries or side-by-side boxes), max 3 columns, maybe 4 if the items are small, use multiple rows if necessary.
4. **Images**: Use image placeholders only: <img>#0</img> for provided source images, <img>$0</img> for suggested search images, or <img>search(short visual search term)</img> to use the first image from a web image search. For #N and $N images, you may optionally provide a display caption after a pipe, for example <img>#0|Moore sculptures on display at Wakehurst</img>.
5. **Timeline**: <div class="timeline"><div class="timeline-item"><span class="date">29th May 2005 OR May 2005 OR 2005</span><span class="event">Event</span></div></div>
6. **Quotes**: <blockquote class="quote">Expert statement...</blockquote>
7. **Custom / Interactive**: To bring a special visual flair, you are HIGHLY encouraged to use custom elements, html with inlined css, data visualizations, long-form storytelling visuals, CSS animated elements, etc. If you need JavaScript for interactivity or dynamic data visualizations, wrap the complete HTML/CSS/JS payload in a custom <interactive>...</interactive> element. Anything inside <interactive> will render in a sandboxed iframe where JavaScript can run; ordinary article HTML outside <interactive> is sanitized and cannot run JavaScript.

### EDITORIAL REQUIREMENTS
- **Image Usage**: Use most relevant provided images, have a hero image near the start, distribute the rest logically. Ignore images that are clearly unnecessary, like branding. Prefer provided source images first, then use suggested search images with <img>$N</img> when they add useful context. Captions are optional; only provide one when it adds useful context.
- **Search Images**: Use search(...) when no listed image fits, and a generic or contextual visual would improve the article, for example <img>search(rolling green hills)</img>. The system will pick the first image from a web image search for that term. Use concise visual search terms, not full sentences.
- **Redundancy**: Don't repeat content on both the main body and the sidebar. Aim for concise informative prose. Include ONLY what is in the source articles, with only minor wording changes and no editorial interpretation. Feel free to move sentences around for better flow when you are combining articles. Only include the same information once, even if it was repeated across multiple sources. You condense into a well crafted and professional overview.

- **Content: Main Body**:
  - Start with a 2-3 paragraph overview.
  - Increase depth and detail as the article progresses (stop if you run out of source material).
  - **Custom Visuals**: If it's a particularly interesting article you'd like to make a feature of, use one or more custom and/or interactive elements.
  - **Timeline (Optional)**: Include a "Timeline" section if the subject has a clear chronological history.
  - **Perspectives (Optional)**: Include a "Perspectives" section at the end using <div class="flexbox-rows"> with <div class="box"> elements to show different viewpoints (e.g., "Critics", "Supporters", "Experts").
  - **Tone**: Professional, objective, and deeply informative.

- **Sidebar: Key Details**:
  - **Maps/Infographics**: If a map or small infographic is available, place it here.
  - **Metadata Table**: Use a table for key data points. Examples:
    - News/Events: "Date", "Location", "Key Figures".
    - Reviews: "Price", "Rating" or "Pros", "Cons".
    - Entities: "Founded", "Headquarters", "Key People".
  - **Quick Highlights**: A <ul> of key takeaways or highlights.

### DATA INPUTS
Available Images:
{images}

Suggested Images From Search:
{suggested_image_list}

Sources with information:
{articles_content}"#
    );

    let entry = run::<ArticleEntry>(&context)
        .await
        .map_err(|e| anyhow!("Article generation failed: {e}"))?;

    let mut search_images = HashMap::new();
    let content_with_images = resolve_image_html(
        &entry.content,
        &available_images,
        &suggested_images,
        &mut search_images,
    )
    .await;
    let sidebar_with_images = resolve_image_html(
        &entry.sidebar,
        &available_images,
        &suggested_images,
        &mut search_images,
    )
    .await;
    let thumbnail = resolve_thumbnail(
        &entry.thumbnail,
        &available_images,
        &suggested_images,
        &mut search_images,
    )
    .await
    .or_else(|| available_images.first().map(|image| image.url.clone()))
    .or_else(|| suggested_images.first().map(|image| image.url.clone()))
    .unwrap_or(entry.thumbnail);
    let background = resolve_thumbnail(
        &entry.background,
        &available_images,
        &suggested_images,
        &mut search_images,
    )
    .await
    .or_else(|| available_images.first().map(|image| image.url.clone()))
    .or_else(|| suggested_images.first().map(|image| image.url.clone()))
    .unwrap_or(entry.background);

    Ok(ArticleEntry {
        content: clean_article_html(&content_with_images),
        sidebar: clean_article_html(&sidebar_with_images),
        thumbnail,
        background,
        ..entry
    })
}

fn clean_article_html(html: &str) -> String {
    let mut sanitizer = ammonia::Builder::default();
    sanitizer.add_generic_attributes(&["class"]);

    let mut cleaned = String::with_capacity(html.len());
    let mut last = 0;
    for captures in INTERACTIVE_RE.captures_iter(html) {
        warn!("Found interactive match: {:?}", captures.get(0));
        let matched = captures.get(0).expect("regex match exists");
        cleaned.push_str(&sanitizer.clean(&html[last..matched.start()]).to_string());
        let interactive_html = captures.get(1).map_or("", |m| m.as_str());
        let document = format!(
            r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src https: data: blob:; media-src https: data: blob:; font-src 'none'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none';">
<style>
:root {{
{theme_css}
}}
{reset_css}
{article_css}
</style>
</head>
<body class="article-content">
{interactive_html}
</body>
</html>"#,
            theme_css = THEME_CSS_VARS,
            reset_css = ARTICLE_IFRAME_RESET_CSS,
            article_css = include_str!("../web/article.css"),
        );
        let srcdoc = html_escape::encode_double_quoted_attribute(&document);
        let _ = write!(
            cleaned,
            r#"<iframe class="interactive-frame" sandbox="allow-scripts" referrerpolicy="no-referrer" loading="lazy" srcdoc="{srcdoc}"></iframe>"#
        );
        last = matched.end();
    }
    cleaned.push_str(&sanitizer.clean(&html[last..]).to_string());
    cleaned
}

async fn resolve_image_html(
    html: &str,
    available: &[CaptionedImage],
    suggested: &[CaptionedImage],
    search_images: &mut HashMap<String, Option<CaptionedImage>>,
) -> String {
    let mut resolved = String::with_capacity(html.len());
    let mut last = 0;

    for captures in IMG_TOKEN_RE.captures_iter(html) {
        let matched = captures.get(0).expect("regex match exists");
        resolved.push_str(&html[last..matched.start()]);
        let token = captures.get(1).map_or("", |m| m.as_str()).trim();
        let (token, display_caption) = image_token_parts(token);
        if let Some(image) = image_for_token(token, available, suggested) {
            resolved.push_str(&image_figure(&image.url, &image.caption, display_caption));
        } else if let Some(image) = image_from_search_token(token, search_images).await {
            resolved.push_str(&image_figure(&image.url, &image.caption, None));
        }
        last = matched.end();
    }

    resolved.push_str(&html[last..]);
    resolved
}

async fn resolve_thumbnail(
    token: &str,
    available: &[CaptionedImage],
    suggested: &[CaptionedImage],
    search_images: &mut HashMap<String, Option<CaptionedImage>>,
) -> Option<String> {
    let (token, _) = image_token_parts(token.trim());
    if let Some(image) = image_for_token(token, available, suggested)
        .or_else(|| image_for_token(token.trim_matches(['[', ']']), available, suggested))
    {
        return Some(image.url.clone());
    }
    if token.starts_with("http") {
        return Some(token.to_string());
    }
    image_from_search_token(token, search_images)
        .await
        .map(|image| image.url)
}

fn image_token_parts(token: &str) -> (&str, Option<&str>) {
    token
        .split_once('|')
        .map_or((token, None), |(token, caption)| {
            (
                token.trim(),
                Some(caption.trim()).filter(|caption| !caption.is_empty()),
            )
        })
}

fn image_for_token<'a>(
    token: &str,
    available: &'a [CaptionedImage],
    suggested: &'a [CaptionedImage],
) -> Option<&'a CaptionedImage> {
    let (prefix, index) = token.split_at_checked(1)?;
    let index = index.parse::<usize>().ok()?;
    match prefix {
        "#" => available.get(index),
        "$" => suggested.get(index),
        _ => None,
    }
}

async fn image_from_search_token(
    token: &str,
    search_images: &mut HashMap<String, Option<CaptionedImage>>,
) -> Option<CaptionedImage> {
    let prompt = token
        .trim_matches(['[', ']'])
        .strip_prefix("search(")?
        .strip_suffix(')')?
        .trim();
    if prompt.is_empty() {
        return None;
    }
    if let Some(image) = search_images.get(prompt) {
        return image.clone();
    }

    info!("Searching images for query: {}", prompt);
    let image = match search_image_urls(prompt, 1).await {
        Ok(mut images) => images.pop().map(|image| CaptionedImage {
            url: image.url,
            caption: image.caption,
        }),
        Err(err) => {
            warn!("Image search failed for '{prompt}': {err:#}");
            None
        }
    };
    search_images.insert(prompt.to_string(), image.clone());
    image
}

fn image_figure(url: &str, alt: &str, caption: Option<&str>) -> String {
    let url = html_escape::encode_double_quoted_attribute(url);
    let alt = html_escape::encode_double_quoted_attribute(alt);
    let caption = caption
        .map(html_escape::encode_text)
        .map(|caption| format!("<figcaption>{caption}</figcaption>"))
        .unwrap_or_default();
    format!(r#"<figure><img src="{url}" alt="{alt}" />{caption}</figure>"#)
}
