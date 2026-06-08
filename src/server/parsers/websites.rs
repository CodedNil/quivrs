use crate::server::parsers::{get_cache_path, get_cached_or_fetch_ext};
use crate::shared::PendingSource;
use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use dom_smoothie::Readability;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use std::{collections::HashMap, path::Path, sync::LazyLock};
use tokio::fs;
use tracing::info;
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
static SEL_JSON: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"script[type="application/json"]"#).unwrap());
static SEL_META: LazyLock<Selector> = LazyLock::new(|| Selector::parse("meta").unwrap());
static SEL_TITLE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("title").unwrap());
static SEL_TIME_DATETIME: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("time[datetime]").unwrap());
static SEL_VISIBLE_DATE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".date, .post-date, .entry-date, .published-at").unwrap());
static SEL_IMG: LazyLock<Selector> = LazyLock::new(|| Selector::parse("img").unwrap());
static SEL_PICTURE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("picture").unwrap());
static SEL_SOURCE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("source").unwrap());
static RE_RESIZE_WIDTH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/resize/(\d+)x").unwrap());
static RE_IMAGE_DIMENSIONS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{3,})x(\d{3,})").unwrap());

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

fn parse_any_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_rfc3339(&format!("{s}:00"))) // Try adding seconds if missing
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z"))
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%d %b %Y")
                .or_else(|_| NaiveDate::parse_from_str(s, "%d %B %Y"))
                .or_else(|_| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|date| date.and_utc())
        })
}

fn is_blacklisted_url(url: &str) -> bool {
    let lower_url = url.to_ascii_lowercase();
    if WEBSITE_BLACKLIST_CONTAINS
        .iter()
        .any(|pattern| lower_url.contains(pattern))
    {
        return true;
    }

    Url::parse(url).is_ok_and(|parsed| {
        parsed.domain().is_some_and(|domain| {
            let domain = domain.trim_start_matches("www.");
            WEBSITE_BLACKLIST_DOMAINS
                .iter()
                .any(|blocked| domain == *blocked || domain.ends_with(&format!(".{blocked}")))
        })
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
    let (metadata, mut images) = collect_page_metadata(&html, base_url.as_ref());

    let get = |keys: &[&str]| -> String {
        keys.iter()
            .find_map(|k| metadata.get(*k))
            .cloned()
            .unwrap_or_default()
    };
    let title = get(&["sl_headline", "og_title", "basic_title"]);
    let summary = get(&["sl_description", "og_description", "basic_description"]);

    let content = Some(get(&["sl_body"])).filter(|s| !s.is_empty()).unwrap_or_else(|| {
        let doc = Html::parse_document(&html);
        let article_sel = Selector::parse("article").unwrap();
        let mut block_content = String::new();

        if let Some(article) = doc.select(&article_sel).next() {
            let text_sel = Selector::parse(r#"div[data-block="text"], div[data-block="headline"], div[data-block="subheadline"], div[data-component="text-block"], div[data-component="headline-block"]"#).unwrap();
            let img_sel =
                Selector::parse(r#"div[data-block="image"], div[data-component="image-block"]"#)
                    .unwrap();

            for block in
                article.select(&Selector::parse(r"div[data-block], div[data-component]").unwrap())
            {
                if text_sel.matches(&block) {
                    for p in block.select(&Selector::parse("p, h1, h2, h3").unwrap()) {
                        let text = p.text().collect::<Vec<_>>().join(" ");
                        if !text.trim().is_empty() {
                            block_content.push_str(&text);
                            block_content.push_str("\n\n");
                        }
                    }
                } else if img_sel.matches(&block) {
                    images.extend(best_images_in(&block, base_url.as_ref()));
                }
            }
        }

        if !block_content.is_empty() {
            block_content.trim().to_string()
        } else if let Ok(mut r) = Readability::new(html.as_str(), Some(url), None) {
            if let Ok(a) = r.parse() {
                let doc = Html::parse_fragment(&a.content);
                images.extend(best_images_in(&doc.root_element(), base_url.as_ref()));
                a.text_content.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    });

    let date = published_hint.or_else(|| extract_published_date(&metadata, &html));

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
    let Some(date) = date else {
        info!("[SKIP] {url}: no date found");
        return Ok(None);
    };

    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut deduped_images: Vec<(String, String)> = Vec::new();
    for (url, caption) in images.into_iter().filter(|(u, _)| usable_image_url(u)) {
        let key = image_dedupe_key(&url);
        if let Some(existing) = seen.get(&key).copied() {
            if image_url_score(&url) > image_url_score(&deduped_images[existing].0) {
                deduped_images[existing] = (url, caption);
            }
        } else {
            seen.insert(key, deduped_images.len());
            deduped_images.push((url, caption));
        }
    }
    deduped_images.sort_by(|a, b| image_url_score(&b.0).total_cmp(&image_url_score(&a.0)));

    let domain = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string();

    Ok(Some(PendingSource {
        url: url.to_string(),

        domain,
        title,
        summary,
        content,
        tags,
        images: deduped_images,
        published: date,

        ..Default::default()
    }))
}

fn decode(s: &str) -> String {
    html_escape::decode_html_entities(s).into_owned()
}

fn resolve_url(base: Option<&Url>, img_url: &str) -> String {
    base.and_then(|b| b.join(img_url).ok())
        .map_or_else(|| img_url.to_string(), |u| u.to_string())
}

fn usable_image_url(url: &str) -> bool {
    !url.is_empty()
        && !url.contains("placeholder")
        && !url.contains("gravatar.com")
        && !url.starts_with("data:")
}

fn best_images_in(root: &ElementRef<'_>, base: Option<&Url>) -> Vec<(String, String)> {
    let pictures = root
        .select(&SEL_PICTURE)
        .filter_map(|picture| {
            let mut candidates = Vec::new();
            let mut caption = String::new();

            for source in picture.select(&SEL_SOURCE) {
                collect_image_candidates(&source, base, &mut candidates);
            }
            for img in picture.select(&SEL_IMG) {
                if caption.is_empty() {
                    caption = img.value().attr("alt").unwrap_or_default().to_string();
                }
                collect_image_candidates(&img, base, &mut candidates);
            }

            candidates
                .into_iter()
                .max_by(|a, b| image_url_score(a).total_cmp(&image_url_score(b)))
                .map(|url| (url, caption))
        })
        .collect::<Vec<_>>();
    if !pictures.is_empty() {
        return pictures;
    }

    root.select(&SEL_IMG)
        .filter_map(|img| {
            let mut candidates = Vec::new();
            collect_image_candidates(&img, base, &mut candidates);
            candidates
                .into_iter()
                .max_by(|a, b| image_url_score(a).total_cmp(&image_url_score(b)))
                .map(|url| (url, img.value().attr("alt").unwrap_or_default().to_string()))
        })
        .collect()
}

fn collect_image_candidates(el: &ElementRef<'_>, base: Option<&Url>, candidates: &mut Vec<String>) {
    let fallback_width = el.value().attr("width").and_then(|w| w.parse::<f32>().ok());

    if let Some(srcset) = el.value().attr("srcset") {
        if let Some((url, _)) = srcset
            .split(',')
            .filter_map(|candidate| {
                let mut parts = candidate.split_whitespace();
                let src = parts.next()?;
                let descriptor = parts.next();
                let url = resolve_url(base, src);
                if !usable_image_url(&url) {
                    return None;
                }
                let width = descriptor
                    .and_then(|d| {
                        d.strip_suffix('w')
                            .and_then(|width| width.parse::<f32>().ok())
                            .or_else(|| {
                                d.strip_suffix('x')
                                    .and_then(|scale| scale.parse::<f32>().ok())
                                    .map(|scale| {
                                        scale
                                            * fallback_width
                                                .or_else(|| infer_resize_width(&url))
                                                .unwrap_or(1000.0)
                                    })
                            })
                    })
                    .or_else(|| infer_resize_width(&url))
                    .unwrap_or_default();
                Some((url, width))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
        {
            candidates.push(url);
        }
        return;
    }

    if let Some(src) = el.value().attr("src") {
        let url = resolve_url(base, src);
        if usable_image_url(&url) {
            candidates.push(url);
        }
    }
}

fn image_url_score(url: &str) -> f32 {
    let width = infer_resize_width(url)
        .or_else(|| infer_largest_dimension_width(url))
        .unwrap_or_default();
    let lower_url = url.to_ascii_lowercase();
    let is_webp_extension = Url::parse(url)
        .ok()
        .and_then(|parsed| {
            Path::new(parsed.path())
                .extension()
                .map(|extension| extension.eq_ignore_ascii_case("webp"))
        })
        .unwrap_or(false);
    let format_bonus = if lower_url.contains("/format/webp") || is_webp_extension {
        0.0
    } else {
        0.1
    };
    width + format_bonus
}

fn image_dedupe_key(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .query_pairs()
                .find_map(|(key, value)| (key == "url").then(|| value.into_owned()))
        })
        .unwrap_or_else(|| url.to_string())
}

fn infer_resize_width(url: &str) -> Option<f32> {
    RE_RESIZE_WIDTH
        .captures(url)?
        .get(1)?
        .as_str()
        .parse::<f32>()
        .ok()
}

fn infer_largest_dimension_width(url: &str) -> Option<f32> {
    RE_IMAGE_DIMENSIONS
        .captures_iter(url)
        .filter_map(|captures| {
            let width = captures.get(1)?.as_str().parse::<u32>().ok()?;
            let height = captures.get(2)?.as_str().parse::<u32>().ok()?;
            (width >= 100 && height >= 100).then_some(width)
        })
        .max()
        .map(|width| width as f32)
}

fn parse_visible_date(html: &str) -> Option<DateTime<Utc>> {
    let doc = Html::parse_document(html);

    doc.select(&SEL_TIME_DATETIME)
        .filter_map(|el| el.value().attr("datetime"))
        .find_map(parse_any_date)
        .or_else(|| {
            doc.select(&SEL_VISIBLE_DATE)
                .map(|el| el.text().collect::<String>())
                .map(|text| text.trim().to_string())
                .find_map(|text| parse_any_date(&text))
        })
}

fn extract_published_date(
    metadata: &HashMap<&'static str, String>,
    html: &str,
) -> Option<DateTime<Utc>> {
    metadata
        .get("sl_date")
        .or_else(|| metadata.get("og_date"))
        .and_then(|s| parse_any_date(s))
        .or_else(|| parse_visible_date(html))
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
        let clean_text = text
            .trim()
            .trim_start_matches("<![CDATA[")
            .trim_end_matches("]]>")
            .trim();
        let decoded_text = decode(clean_text);

        if let Ok(v) = serde_json::from_str::<Value>(&decoded_text) {
            collect_jsonld(&v, &mut m, &mut images, base);
        } else if let Ok(v) = serde_json::from_str::<Value>(clean_text) {
            collect_jsonld(&v, &mut m, &mut images, base);
        }
    }

    for el in doc.select(&SEL_JSON) {
        let id = el.value().attr("id").unwrap_or_default();
        if id == "ng-state" || id.contains("state") {
            let text = el.text().collect::<String>();
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                collect_jsonld(&v, &mut m, &mut images, base);
            }
        }
    }

    for el in doc.select(&SEL_META) {
        let name = el
            .value()
            .attr("name")
            .or_else(|| el.value().attr("property"))
            .map(str::to_ascii_lowercase);
        let content = el.value().attr("content");
        match (name.as_deref(), content) {
            (Some("og:title" | "twitter:title"), Some(v)) => {
                let d = decode(v);
                if d.len() > MIN_TITLE_LEN {
                    m.entry("og_title").or_insert(d);
                }
            }
            (Some("og:description" | "twitter:description"), Some(v)) => {
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
            (Some("og:article:published_time" | "article:published_time"), Some(v)) => {
                m.entry("og_date").or_insert_with(|| v.to_string());
            }
            (Some("parsely-tags"), Some(v)) => {
                m.entry("parsely_tags").or_insert_with(|| v.to_string());
            }
            _ => {}
        }
    }

    if let Some(img_url) = m.remove("og_image") {
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
        .or_else(|| obj.get("published_at"))
        .or_else(|| obj.get("updated_at"))
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
