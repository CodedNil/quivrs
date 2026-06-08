use crate::server::parsers::usable_article_url;
use anyhow::Result;
use chrono::{DateTime, Utc};
use duckduckgo::{browser::Browser, response::ImageResult, user_agents};
use std::collections::HashSet;
use tracing::{info, warn};
use url::Url;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub url: String,
    pub published: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct SearchImageResult {
    pub url: String,
    pub caption: String,
}

/// Searches `DuckDuckGo` news results and returns article URLs with result dates.
pub async fn search_article_urls(query: &str) -> Result<Vec<SearchResult>> {
    let browser = Browser::new();
    let results = browser
        .news(
            query,
            "us-en",
            true,
            Some(8),
            user_agents::get("firefox").unwrap_or("Mozilla/5.0"),
        )
        .await?;

    let mut seen = HashSet::new();
    let mut urls = Vec::new();

    for result in results {
        let Some(url) = usable_article_url(&result.url).filter(|url| {
            Url::parse(url).is_ok_and(|parsed| {
                !parsed.domain().is_some_and(|domain| {
                    domain.ends_with("duckduckgo.com")
                        || domain.ends_with("youtube.com")
                        || domain.ends_with("youtu.be")
                }) && !std::path::Path::new(parsed.path())
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            })
        }) else {
            continue;
        };

        if seen.insert(url.clone()) {
            urls.push(SearchResult {
                published: {
                    DateTime::parse_from_rfc3339(&result.date)
                        .map(|date| date.with_timezone(&Utc))
                        .ok()
                },
                url,
            });
        }
    }

    if urls.is_empty() {
        warn!("Web search returned no article URLs for query: {query}");
    }

    Ok(urls)
}

/// Searches `DuckDuckGo` image results and returns direct image URLs with captions.
pub async fn search_image_urls(query: &str, limit: usize) -> Result<Vec<SearchImageResult>> {
    info!("Searching images for query: {query}");
    let browser = Browser::new();
    let results = browser
        .images(
            query,
            "us-en",
            true,
            Some(limit),
            user_agents::get("firefox").unwrap_or("Mozilla/5.0"),
        )
        .await?;

    let mut seen = HashSet::new();
    let mut images = Vec::new();

    for result in results {
        let Some(url) = image_url(&result) else {
            continue;
        };

        if seen.insert(image_dedupe_key(&url)) {
            images.push(SearchImageResult {
                url,
                caption: image_caption(&result, query),
            });
        }
    }

    if images.is_empty() {
        warn!("Image search returned no images for query: {query}");
    }

    Ok(images)
}

fn image_url(result: &ImageResult) -> Option<String> {
    [&result.image, &result.thumbnail]
        .into_iter()
        .find(|url| {
            !url.trim().is_empty()
                && Url::parse(url).is_ok_and(|parsed| {
                    matches!(parsed.scheme(), "http" | "https")
                        && !parsed.domain().is_some_and(|domain| {
                            domain.ends_with("duckduckgo.com")
                                || domain.ends_with("youtube.com")
                                || domain.ends_with("youtu.be")
                        })
                })
        })
        .cloned()
}

fn image_caption(result: &ImageResult, query: &str) -> String {
    let title = result.title.trim();
    let source = result.source.trim();
    match (title.is_empty(), source.is_empty()) {
        (false, false) => format!("{title} ({source})"),
        (false, true) => title.to_string(),
        (true, false) => format!("{query} image from {source}"),
        (true, true) => format!("{query} image result"),
    }
}

pub fn image_dedupe_key(url: &str) -> String {
    Url::parse(url).map_or_else(
        |_| url.to_ascii_lowercase(),
        |mut parsed| {
            parsed.set_query(None);
            parsed.set_fragment(None);
            parsed.to_string().to_ascii_lowercase()
        },
    )
}
