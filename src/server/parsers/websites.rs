use crate::server::parsers::{get_cache_path, get_cached_or_fetch_ext};
use crate::shared::ArticleSource;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use dom_smoothie::Readability;
use scraper::{Html, Selector};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};
use tokio::fs;
use tracing::{debug, info};
use url::Url;

const MIN_SUMMARY_LEN: usize = 30;
const MIN_TITLE_LEN: usize = 6;
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

static SEL_JSONLD: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"script[type="application/ld+json"]"#).unwrap());
static SEL_META: LazyLock<Selector> = LazyLock::new(|| Selector::parse("meta").unwrap());
static SEL_TITLE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("title").unwrap());

const WEBSITE_BLACKLIST: &[&str] = &["bbc.com/news/videos", "bbc.co.uk/news/videos"];

pub async fn fetch_source_content(url: &str) -> Result<Option<ArticleSource>> {
    if WEBSITE_BLACKLIST.iter().any(|s| url.contains(s)) {
        return Ok(None);
    }

    let cache_path = get_cache_path(url, "html");
    let html = get_cached_or_fetch_ext(url, "html").await?;

    let base_url = Url::parse(url).ok();
    let (metadata, mut images) = collect_page_metadata(&html, base_url.as_ref());

    let get = |keys: &[&str]| -> String {
        keys.iter()
            .find_map(|k| metadata.get(*k))
            .cloned()
            .unwrap_or_default()
    };
    let title = get(&["sl_headline", "og_title", "basic_title"]);
    let summary = get(&["sl_description", "og_description", "basic_description"]);
    let content = Some(get(&["sl_body"]))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let doc = Html::parse_document(&html);
            let article_sel = Selector::parse("article").unwrap();
            if let Some(article) = doc.select(&article_sel).next() {
                let mut block_content = String::new();
                let mut block_images = Vec::new();

                let text_sel = Selector::parse(r#"div[data-block="text"], div[data-block="headline"], div[data-block="subheadline"], div[data-component="text-block"], div[data-component="headline-block"]"#).unwrap();
                let img_sel = Selector::parse(r#"div[data-block="image"], div[data-component="image-block"]"#).unwrap();

                for block in article.select(&Selector::parse(r"div[data-block], div[data-component]").unwrap()) {
                    if text_sel.matches(&block) {
                        for p in block.select(&Selector::parse("p, h1, h2, h3").unwrap()) {
                            let text = p.text().collect::<Vec<_>>().join(" ");
                            if !text.trim().is_empty() {
                                block_content.push_str(&text);
                                block_content.push_str("\n\n");
                            }
                        }
                    } else if img_sel.matches(&block) {
                        for img in block.select(&Selector::parse("img").unwrap()) {
                            let src = img.value().attr("srcset")
                                .and_then(|srcset| {
                                    srcset.split(',')
                                        .filter_map(|s| {
                                            let parts: Vec<_> = s.split_whitespace().collect();
                                            if parts.is_empty() { return None; }
                                            let url = parts[0];
                                            let width = parts.get(1)
                                                .and_then(|w| w.strip_suffix('w'))
                                                .and_then(|w| w.parse::<u32>().ok())
                                                .unwrap_or(0);
                                            Some((url, width))
                                        })
                                        .max_by_key(|&(_, width)| width)
                                        .map(|(url, _)| url.to_string())
                                })
                                .or_else(|| img.value().attr("src").map(String::from));

                            if let Some(src) = src {
                                if src.contains("placeholder") {
                                    continue;
                                }
                                let alt = img.value().attr("alt").unwrap_or_default().to_string();
                                block_images.push((resolve_url(base_url.as_ref(), &src), alt));
                            }
                        }
                    }
                }

                if !block_content.is_empty() {
                    images.clear();
                    images.extend(block_images);
                    return block_content.trim().to_string();
                }
            }

            Readability::new(html.as_str(), Some(url), None)
                .ok()
                .and_then(|mut r| r.parse().ok())
                .map(|a| {
                    debug!("Used fallback parser for url: {}", url);
                    let doc = Html::parse_fragment(&a.content);
                    let sel = Selector::parse("img").unwrap();
                    images.clear();
                    for img in doc.select(&sel) {
                        let src = img
                            .value()
                            .attr("srcset")
                            .and_then(|srcset| {
                                srcset
                                    .split(',')
                                    .filter_map(|s| {
                                        let parts: Vec<_> = s.split_whitespace().collect();
                                        if parts.is_empty() {
                                            return None;
                                        }
                                        let url = parts[0];
                                        let width = parts
                                            .get(1)
                                            .and_then(|w| w.strip_suffix('w'))
                                            .and_then(|w| w.parse::<u32>().ok())
                                            .unwrap_or(0);
                                        Some((url, width))
                                    })
                                    .max_by_key(|&(_, width)| width)
                                    .map(|(url, _)| url.to_string())
                            })
                            .or_else(|| img.value().attr("src").map(String::from));

                        if let Some(src) = src {
                            let alt = img.value().attr("alt").unwrap_or_default().to_string();
                            images.push((resolve_url(base_url.as_ref(), &src), alt));
                        }
                    }
                    a.text_content.to_string()
                })
                .unwrap_or_default()
        });
    let date = metadata
        .get("sl_date")
        .or_else(|| metadata.get("og_date"))
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map_or_else(Utc::now, |d| d.with_timezone(&Utc));
    let tags = metadata
        .get("sl_keywords")
        .or_else(|| metadata.get("parsely_tags"))
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .or_else(|| metadata.get("sl_section").map(|s| vec![s.clone()]))
        .unwrap_or_default();

    if !title.is_empty() && BOT_TITLES.iter().any(|t| title.to_lowercase().contains(t)) {
        let _ = fs::remove_file(&cache_path).await;
        return Err(anyhow!("bot-protection page: {title}"));
    }
    if title.is_empty() {
        info!("[SKIP] {url}: empty title");
        return Ok(None);
    }
    if summary.is_empty() {
        info!("[SKIP] {url}: empty summary (title: {title})");
        return Ok(None);
    }
    if content.is_empty() {
        info!("[SKIP] {url}: empty content (title: {title})");
        return Ok(None);
    }
    if title == summary {
        info!("[SKIP] {url}: title matches summary");
        return Ok(None);
    }

    let mut seen = HashSet::new();
    let images = images
        .into_iter()
        .filter(|(u, _)| {
            !u.is_empty()
                && !u.contains("placeholder")
                && !u.contains("gravatar.com")
                && seen.insert(u.clone())
        })
        .map(|(u, c)| format!("{u}|{c}"))
        .collect();

    let source = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string();

    Ok(Some(ArticleSource {
        url: url.to_string(),
        source,
        title,
        summary,
        content,
        tags,
        images,
        published: date,
    }))
}

fn decode(s: &str) -> String {
    html_escape::decode_html_entities(s).into_owned()
}

fn resolve_url(base: Option<&Url>, img_url: &str) -> String {
    base.and_then(|b| b.join(img_url).ok())
        .map_or_else(|| img_url.to_string(), |u| u.to_string())
}

fn collect_page_metadata(
    html: &str,
    base: Option<&Url>,
) -> (HashMap<&'static str, String>, Vec<(String, String)>) {
    let doc = Html::parse_document(html);
    let mut m: HashMap<&'static str, String> = HashMap::new();
    let mut images: Vec<(String, String)> = vec![];

    for el in doc.select(&SEL_JSONLD) {
        let text = el.text().collect::<String>();
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            collect_jsonld(&v, &mut m, &mut images, base);
        }
    }

    for el in doc.select(&SEL_META) {
        let name = el
            .value()
            .attr("name")
            .or_else(|| el.value().attr("property"));
        let content = el.value().attr("content");
        match (name, content) {
            (Some("og:title"), Some(v)) => {
                let d = decode(v);
                if d.len() > MIN_TITLE_LEN {
                    m.entry("og_title").or_insert(d);
                }
            }
            (Some("og:description"), Some(v)) => {
                let d = decode(v);
                if d.len() > MIN_SUMMARY_LEN {
                    m.entry("og_description").or_insert(d);
                }
            }
            (Some("description"), Some(v)) => {
                let d = decode(v);
                if d.len() > MIN_SUMMARY_LEN {
                    m.entry("basic_description").or_insert(d);
                }
            }
            (Some("og:image"), Some(v)) => {
                m.entry("og_image").or_insert_with(|| resolve_url(base, v));
            }
            (Some("og:image:alt"), Some(v)) => {
                m.entry("og_image_alt").or_insert_with(|| decode(v));
            }
            (Some("og:article:published_time"), Some(v)) => {
                m.entry("og_date").or_insert_with(|| v.to_string());
            }
            (Some("parsely-tags"), Some(v)) => {
                m.entry("parsely_tags").or_insert_with(|| v.to_string());
            }
            _ => {}
        }
    }

    if images.is_empty()
        && let Some(img_url) = m.remove("og_image")
    {
        images.push((img_url, m.remove("og_image_alt").unwrap_or_default()));
    }

    if let Some(el) = doc.select(&SEL_TITLE).next() {
        let raw = el.text().collect::<String>();
        let clean = raw
            .rfind(" | ")
            .or_else(|| raw.rfind(" \u{2014} "))
            .or_else(|| raw.rfind(" - "))
            .map_or(raw.as_str(), |pos| &raw[..pos])
            .trim();
        if clean.len() > MIN_TITLE_LEN {
            m.entry("basic_title").or_insert_with(|| decode(clean));
        }
    }

    (m, images)
}

fn collect_jsonld(
    v: &Value,
    m: &mut HashMap<&'static str, String>,
    images: &mut Vec<(String, String)>,
    base: Option<&Url>,
) {
    match v {
        Value::Array(arr) => arr
            .iter()
            .for_each(|item| collect_jsonld(item, m, images, base)),
        obj => match obj.get("@graph") {
            Some(Value::Array(graph)) => graph
                .iter()
                .for_each(|item| collect_jsonld_object(item, m, images, base)),
            _ => collect_jsonld_object(obj, m, images, base),
        },
    }
}

fn collect_jsonld_object(
    obj: &Value,
    m: &mut HashMap<&'static str, String>,
    images: &mut Vec<(String, String)>,
    base: Option<&Url>,
) {
    if let Some(v) = obj
        .get("headline")
        .or_else(|| obj.get("name"))
        .and_then(Value::as_str)
    {
        let d = decode(v);
        if d.len() > MIN_TITLE_LEN {
            m.entry("sl_headline").or_insert(d);
        }
    }
    if let Some(v) = obj
        .get("description")
        .or_else(|| obj.get("alternativeHeadline"))
        .and_then(Value::as_str)
    {
        let d = decode(v);
        if d.len() > MIN_SUMMARY_LEN {
            m.entry("sl_description").or_insert(d);
        }
    }
    if let Some(v) = obj
        .get("articleBody")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        m.entry("sl_body").or_insert_with(|| v.to_string());
    }
    if let Some(kw) = obj.get("keywords") {
        let csv = match kw {
            Value::Array(arr) => arr
                .iter()
                .filter_map(Value::as_str)
                .map(|s| decode(s.trim()))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(","),
            Value::String(s) => s.clone(),
            _ => String::new(),
        };
        if !csv.is_empty() {
            m.entry("sl_keywords").or_insert(csv);
        }
    }
    if let Some(v) = obj
        .get("articleSection")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        m.entry("sl_section").or_insert_with(|| decode(v));
    }
    if let Some(v) = obj
        .get("datePublished")
        .or_else(|| obj.get("dateModified"))
        .and_then(Value::as_str)
    {
        m.entry("sl_date").or_insert_with(|| v.to_string());
    }
    if images.is_empty()
        && let Some(img) = obj.get("image").or_else(|| obj.get("thumbnail"))
    {
        collect_images(img, images, base);
    }
}

fn collect_images(v: &Value, out: &mut Vec<(String, String)>, base: Option<&Url>) {
    match v {
        Value::String(url) if !url.is_empty() => out.push((resolve_url(base, url), String::new())),
        Value::Object(obj) => {
            if let Some(url) = obj
                .get("url")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                out.push((
                    resolve_url(base, url),
                    obj.get("caption")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ));
            }
        }
        Value::Array(arr) => arr.iter().for_each(|item| collect_images(item, out, base)),
        _ => {}
    }
}
