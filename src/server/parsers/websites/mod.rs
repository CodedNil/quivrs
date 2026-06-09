mod content;
mod date;
mod images;
mod summary;
mod tags;
mod title;

#[cfg(test)]
mod tests;

use crate::server::parsers::{get_cache_path, get_cached_or_fetch_ext};
use crate::shared::PendingSource;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;
use tokio::fs;
use tracing::info;
use url::Url;

pub struct PageData {
    html: String,
    jsonld: Vec<Value>,
    meta: Vec<MetaTag>,
}

pub struct MetaTag {
    name: String,
    content: String,
}

const BOT_TITLES: &[&str] = &[
    "access denied",
    "security verification",
    "verifying device",
    "just a moment",
    "checking your browser",
    "attention required",
    "please verify",
    "403 forbidden",
    "enable javascript",
];
const BOT_BODY_MARKERS: &[&str] = &[
    "captcha-delivery.com",
    "x-datadome",
    "please enable js and disable any ad blocker",
];

static SEL_JSONLD: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"script[type="application/ld+json"]"#).unwrap());
static SEL_JSON: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"script[type="application/json"]"#).unwrap());
static SEL_META: LazyLock<Selector> = LazyLock::new(|| Selector::parse("meta").unwrap());
const WEBSITE_BLACKLIST_DOMAINS: &[&str] = &[
    "reddit.com",
    "lobste.rs",
    "github.com",
    "codeberg.org",
    "ycombinator.com",
    "msn.com",
    "wn.com",
];

const WEBSITE_BLACKLIST_CONTAINS: &[&str] = &[
    "bbc.com/news/videos",
    "bbc.co.uk/news/videos",
    "bbc.co.uk/iplayer",
    "bbc.co.uk/sounds",
    "linkedin.com/pulse",
    ".pdf",
];

fn is_blacklisted_url(url: &str) -> bool {
    let lower_url = url.to_ascii_lowercase();
    if WEBSITE_BLACKLIST_CONTAINS
        .iter()
        .any(|pattern| lower_url.contains(pattern))
    {
        return true;
    }

    clean_domain(url).is_some_and(|domain| {
        WEBSITE_BLACKLIST_DOMAINS
            .iter()
            .any(|blocked| domain == *blocked || domain.ends_with(&format!(".{blocked}")))
    })
}

pub async fn fetch_source_content(
    url: &str,
    published_hint: Option<DateTime<Utc>>,
) -> Result<Option<PendingSource>> {
    if is_blacklisted_url(url) {
        return Ok(None);
    }

    let cache_path = get_cache_path(url, "html");
    let html = get_cached_or_fetch_ext(url, "html").await?;

    let base_url = Url::parse(url).ok();
    let page = PageData::parse(html);

    let Some(title) = title::parse(&page) else {
        info!("[SKIP] {url}: empty title");
        return Ok(None);
    };
    if is_bot_protection_page(&title, &page.html) {
        let _ = fs::remove_file(&cache_path).await;
        return Err(anyhow!("bot-protection page: {title}"));
    }
    let Some(summary) = summary::parse(&page) else {
        info!("[SKIP] {url}: empty summary (title: {title})");
        return Ok(None);
    };

    let content = content::parse(&page, url).unwrap_or_default();
    if content.is_empty() {
        info!("[SKIP] {url}: empty content (title: {title})");
        return Ok(None);
    }
    if title == summary {
        info!("[SKIP] {url}: title matches summary");
        return Ok(None);
    }
    let date = published_hint.or_else(|| date::parse(&page));
    let Some(date) = date else {
        info!("[SKIP] {url}: no date found");
        return Ok(None);
    };

    let images = images::parse(&page, base_url.as_ref());
    let tags = tags::parse(&page);

    Ok(Some(PendingSource {
        url: url.to_string(),

        domain: clean_domain(url).unwrap_or_default(),
        title,
        summary,
        content,
        tags,
        images,
        published: date,

        ..Default::default()
    }))
}

fn clean_domain(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|url| url.domain().map(str::to_string))
        .map(|domain| domain.trim_start_matches("www.").to_string())
}

fn is_bot_protection_page(title: &str, html: &str) -> bool {
    let lower_title = title.to_ascii_lowercase();
    let lower_html = html.to_ascii_lowercase();
    BOT_TITLES.iter().any(|marker| lower_title.contains(marker))
        || BOT_BODY_MARKERS
            .iter()
            .any(|marker| lower_html.contains(marker))
}

fn decode(s: &str) -> String {
    html_escape::decode_html_entities(s).into_owned()
}

impl PageData {
    fn parse(html: String) -> Self {
        let doc = Html::parse_document(&html);
        let mut jsonld = Vec::new();
        let mut meta = Vec::new();

        for el in doc.select(&SEL_JSONLD) {
            let text = el.text().collect::<String>();
            let clean_text = text
                .trim()
                .trim_start_matches("<![CDATA[")
                .trim_end_matches("]]>")
                .trim();
            let decoded_text = decode(clean_text);

            for text in [decoded_text.as_str(), clean_text] {
                if let Ok(v) = serde_json::from_str::<Value>(text) {
                    jsonld.push(v);
                    break;
                }
            }
        }

        for el in doc.select(&SEL_JSON) {
            let id = el.value().attr("id").unwrap_or_default();
            if id == "ng-state" || id.contains("state") {
                let text = el.text().collect::<String>();
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    jsonld.push(v);
                }
            }
        }

        for el in doc.select(&SEL_META) {
            if let (Some(name), Some(content)) = (
                el.value()
                    .attr("name")
                    .or_else(|| el.value().attr("property"))
                    .map(str::to_ascii_lowercase),
                el.value().attr("content"),
            ) {
                meta.push(MetaTag {
                    name,
                    content: content.to_string(),
                });
            }
        }

        Self { html, jsonld, meta }
    }
}

pub fn jsonld_values(v: &Value) -> Box<dyn Iterator<Item = &Value> + '_> {
    match v {
        Value::Array(arr) => Box::new(arr.iter().flat_map(jsonld_values)),
        obj => match obj.get("@graph").and_then(Value::as_array) {
            Some(graph) => Box::new(graph.iter()),
            None => Box::new(std::iter::once(obj)),
        },
    }
}
