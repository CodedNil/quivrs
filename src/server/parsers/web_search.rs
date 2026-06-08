use crate::server::{
    HTTP_CLIENT,
    parsers::{normalize_image_url, usable_article_url},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use duckduckgo::{browser::Browser, user_agents};
use std::{collections::HashSet, path::Path};
use tracing::warn;
use url::Url;

const LOCALE: &str = "us-en";
const ARTICLE_LIMIT: usize = 8;
const FALLBACK_USER_AGENT: &str = "Mozilla/5.0";
const BLOCKED_DOMAINS: &[&str] = &["duckduckgo.com", "youtube.com", "youtu.be"];
const WATERMARK_IMAGE_DOMAINS: &[&str] = &[
    "alamy.com",
    "istockphoto.com",
    "shutterstock.com",
    "dreamstime.com",
    "123rf.com",
    "depositphotos.com",
];
const WATERMARK_IMAGE_TEXT: &[&str] = &[
    "alamy",
    "istock",
    "gettyimages",
    "shutterstock",
    "dreamstime",
    "123rf",
    "depositphotos",
    "stock photo",
    "stock photography",
    "royalty-free",
];

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
    let user_agent = user_agents::get("firefox").unwrap_or(FALLBACK_USER_AGENT);
    let results = browser
        .news(query, LOCALE, true, Some(ARTICLE_LIMIT), user_agent)
        .await?;

    let mut seen = HashSet::new();
    let mut urls = Vec::new();

    for result in results {
        let Some(url) = usable_article_url(&result.url).filter(|url| {
            Url::parse(url).is_ok_and(|parsed| {
                matches!(parsed.scheme(), "http" | "https")
                    && !parsed.domain().is_some_and(is_blocked_domain)
                    && !Path::new(parsed.path())
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            })
        }) else {
            continue;
        };

        if !seen.insert(url.clone()) {
            continue;
        }

        urls.push(SearchResult {
            published: DateTime::parse_from_rfc3339(&result.date)
                .map(|date| date.with_timezone(&Utc))
                .ok(),
            url,
        });
    }

    if urls.is_empty() {
        warn!("Web search returned no article URLs for query: {query}");
    }

    Ok(urls)
}

/// Searches `DuckDuckGo` image results and returns direct image URLs with captions.
pub async fn search_image_urls(query: &str, limit: usize) -> Result<Vec<SearchImageResult>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let browser = Browser::new();
    let user_agent = user_agents::get("firefox").unwrap_or(FALLBACK_USER_AGENT);
    let results = browser
        .images(query, LOCALE, true, Some((limit * 10).max(25)), user_agent)
        .await?;

    let mut seen = HashSet::new();
    let mut images = Vec::new();

    for result in results {
        let result_text = [
            result.title.as_str(),
            result.source.as_str(),
            result.url.as_str(),
        ]
        .join(" ")
        .to_ascii_lowercase();
        if WATERMARK_IMAGE_TEXT
            .iter()
            .any(|pattern| result_text.contains(pattern))
        {
            continue;
        }
        let candidate_text = [result.image.as_str(), result.thumbnail.as_str()]
            .join(" ")
            .to_ascii_lowercase();
        if WATERMARK_IMAGE_TEXT
            .iter()
            .any(|pattern| candidate_text.contains(pattern))
        {
            continue;
        }

        let Some(url) = [&result.image, &result.thumbnail]
            .into_iter()
            .find_map(|url| {
                let url = normalize_image_url(url);
                (!url.is_empty()
                    && Url::parse(&url).is_ok_and(|parsed| {
                        matches!(parsed.scheme(), "http" | "https")
                            && !parsed.domain().is_some_and(is_blocked_image_domain)
                    }))
                .then_some(url)
            })
        else {
            continue;
        };

        let Ok(response) = HTTP_CLIENT.head(&url).send().await else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        if response
            .headers()
            .get("Cross-Origin-Resource-Policy")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|policy| {
                policy.eq_ignore_ascii_case("same-origin")
                    || policy.eq_ignore_ascii_case("same-site")
            })
        {
            continue;
        }

        if seen.insert(image_dedupe_key(&url)) {
            images.push(SearchImageResult {
                url,
                caption: result.title.trim().to_string(),
            });
            if images.len() >= limit {
                break;
            }
        }
    }

    if images.is_empty() {
        warn!("Image search returned no images for query: {query}");
    }

    Ok(images)
}

fn is_blocked_domain(domain: &str) -> bool {
    domain_matches(domain, BLOCKED_DOMAINS)
}

fn is_blocked_image_domain(domain: &str) -> bool {
    is_blocked_domain(domain)
        || domain_matches(domain, WATERMARK_IMAGE_DOMAINS)
        || domain.split('.').any(|label| label == "gettyimages")
}

fn domain_matches(domain: &str, blocked_domains: &[&str]) -> bool {
    blocked_domains.iter().any(|blocked| {
        domain == *blocked
            || domain
                .strip_suffix(blocked)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
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
