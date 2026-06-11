pub mod feeds;
pub mod social;
pub mod web_search;
pub mod websites;

use crate::server::HTTP_CLIENT;
use crate::shared::PendingSource;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use social::fetch_social_content;
use std::fmt::Write;
use std::path::PathBuf;
use tokio::fs;
use url::Url;
use url_normalize::{Options as NormalizeOptions, QueryFilter, RemoveQueryParameters};
use websites::fetch_source_content;

const MAX_ARTICLE_AGE_DAYS: i64 = 4;

pub async fn fetch_page_content(url: &str) -> Result<Option<PendingSource>> {
    fetch_page_content_with_hint(url, None).await
}

pub async fn fetch_page_content_with_hint(
    url: &str,
    published_hint: Option<DateTime<Utc>>,
) -> Result<Option<PendingSource>> {
    let result = if is_host_or_subdomain(url, &["twitter.com", "x.com", "bsky.app"]) {
        fetch_social_content(url).await
    } else {
        fetch_source_content(url, published_hint).await
    };
    // Drop result if older than a week
    if let Ok(Some(result)) = &result
        && is_article_too_old(result.published)
    {
        return Ok(None);
    }
    // Drop result if content is less than 300 characters
    if let Ok(Some(result)) = &result
        && result.content.len() < 400
    {
        return Ok(None);
    }
    result
}

fn is_host_or_subdomain(url: &str, domains: &[&str]) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| {
            domains
                .iter()
                .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        })
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn get_cache_path(url: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join("quivrs").join(format!(
        "{}.{}",
        sha256_hex(url.as_bytes()),
        extension
    ))
}

async fn get_cached_or_fetch_ext(url: &str, ext: &str) -> Result<String> {
    let cache_path = get_cache_path(url, ext);

    if let Ok(bytes) = fs::read(&cache_path).await {
        return Ok(String::from_utf8_lossy(&bytes).into_owned());
    }

    let text = HTTP_CLIENT.get(url).send().await?.text().await?;
    if let Some(dir) = cache_path.parent()
        && fs::create_dir_all(dir).await.is_ok()
    {
        let _ = fs::write(&cache_path, text.as_bytes()).await;
    }
    Ok(text)
}

fn is_article_too_old(published: chrono::DateTime<Utc>) -> bool {
    published < Utc::now() - Duration::days(MAX_ARTICLE_AGE_DAYS)
}

/// Only strips known tracking parameters while keeping meaningful query params.
fn normalize_article_url(url: &str) -> String {
    const TRACKING_PARAMS: &[&str] = &[
        "ref",
        "source",
        "fbclid",
        "gclid",
        "msclkid",
        "yclid",
        "igshid",
        "app-referrer",
        "ito",
    ];

    url_normalize::normalize_url(
        url,
        &NormalizeOptions {
            force_https: true,
            strip_hash: true,
            remove_query_parameters: RemoveQueryParameters::List(vec![QueryFilter::Predicate(
                Box::new(|key: &str| {
                    let lower = key.to_ascii_lowercase();
                    TRACKING_PARAMS.contains(&key)
                        || lower.starts_with("utm_")
                        || lower.starts_with("at_")
                        || lower.starts_with("mc_")
                }),
            )]),
            ..Default::default()
        },
    )
    .unwrap_or_else(|_| url.to_string())
}

fn normalize_image_url(url: &str) -> String {
    url_normalize::normalize_url(
        url.trim(),
        &NormalizeOptions {
            force_https: true,
            sort_query_parameters: false, // Protons CDN is picky about order
            remove_trailing_slash: false,
            ..Default::default()
        },
    )
    .unwrap_or_else(|_| url.to_string())
}

fn is_base_url(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|parsed| {
        let path = parsed.path();
        path == "/" || path.is_empty()
    })
}

fn usable_article_url(url: &str) -> Option<String> {
    let normalized = normalize_article_url(url);
    (!is_base_url(&normalized)).then_some(normalized)
}
