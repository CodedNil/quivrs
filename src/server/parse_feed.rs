use crate::{server::HTTP_CLIENT, shared::ArticleSource};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::Deserialize;
use tracing::warn;
use url_normalize::{Options as NormalizeOptions, RemoveQueryParameters};

const MEDIA_NS: &str = "http://search.yahoo.com/mrss/";
const CONTENT_NS: &str = "http://purl.org/rss/1.0/modules/content/";
const DC_NS: &str = "http://purl.org/dc/elements/1.1/";

#[derive(Debug)]
struct ParsedEntry {
    url: String,
    title: String,
    content: String,
    published: DateTime<Utc>,
    image: Option<String>,
    image_description: Option<String>,
}

pub async fn scan_feed(url_rss: &str) -> Result<Vec<ArticleSource>> {
    let bytes = HTTP_CLIENT.get(url_rss).send().await?.bytes().await?;

    let safe_filename = url_rss
        .replace("https://", "")
        .replace("http://", "")
        .replace(['/', ':'], "_");
    std::fs::create_dir_all("tmp").ok();
    std::fs::write(format!("tmp/{safe_filename}"), bytes.as_ref()).ok();

    let entries = parse_feed(bytes.as_ref())?;
    std::fs::write(
        format!("tmp/{safe_filename}.parsed"),
        format!("{entries:#?}"),
    )
    .ok();

    Ok(entries
        .into_iter()
        .filter(|e| {
            !(e.url.contains("bbc.co.uk/iplayer")
                || e.url.contains("bbc.co.uk/sounds")
                || (e.url.contains("reddit.com") && e.url.contains("/comments"))
                || e.url.contains("v.redd.it/"))
        })
        .map(|e| ArticleSource {
            url: e.url,
            title: e.title,
            summary: e.content,
            image: e.image,
            image_description: e.image_description,
            published: e.published,
        })
        .collect())
}

fn parse_feed(bytes: &[u8]) -> Result<Vec<ParsedEntry>> {
    let first_non_ws = bytes
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .unwrap_or(0);
    if bytes.get(first_non_ws) == Some(&b'{') {
        return parse_json_feed(bytes);
    }
    parse_xml_feed(bytes)
}

fn parse_xml_feed(bytes: &[u8]) -> Result<Vec<ParsedEntry>> {
    let text = std::str::from_utf8(bytes).map_err(|_| anyhow!("Feed is not valid UTF-8"))?;
    let doc = roxmltree::Document::parse(text).map_err(|e| anyhow!("XML parse error: {e}"))?;
    let root = doc.root_element();
    match root.tag_name().name() {
        "rss" => {
            let channel = root
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "channel")
                .ok_or_else(|| anyhow!("RSS feed missing <channel>"))?;
            Ok(rss_items(channel.children()))
        }
        "RDF" => Ok(rss_items(root.children())),
        "feed" => Ok(parse_atom(root)),
        name => Err(anyhow!("Unknown feed root element: <{name}>")),
    }
}

fn rss_items<'a>(nodes: impl Iterator<Item = roxmltree::Node<'a, 'a>>) -> Vec<ParsedEntry> {
    nodes
        .filter(|n| n.is_element() && n.tag_name().name() == "item")
        .filter_map(parse_rss_item)
        .collect()
}

// Handles both RSS 2.0 (<pubDate>) and RSS 1.0 (<dc:date>, rdf:about URL fallback).
fn parse_rss_item(item: roxmltree::Node) -> Option<ParsedEntry> {
    let thread_url = child_text(item, "link", None)
        .map(|s| s.trim().to_string())
        .or_else(|| {
            item.attributes()
                .find(|a| a.name() == "about")
                .map(|a| a.value().to_string())
        })?;

    let title =
        child_text(item, "title", None).map_or_else(|| "Untitled".into(), |s| s.trim().to_string());

    let html = join_content([
        child_text(item, "encoded", Some(CONTENT_NS)),
        child_text(item, "description", None),
    ]);

    let url = if thread_url.contains("reddit.com/r/") {
        normalize_article_url(&extract_reddit_url(&html)?)
    } else {
        normalize_article_url(&thread_url)
    };

    let published = child_text(item, "pubDate", None)
        .or_else(|| child_text(item, "date", Some(DC_NS)))
        .and_then(|s| parse_datetime(&s))
        .unwrap_or_else(Utc::now);

    let (image, image_description) = find_image(item, &html);
    let content = strip_html(&html);
    Some(ParsedEntry {
        url,
        title,
        content,
        published,
        image,
        image_description,
    })
}

fn parse_atom(root: roxmltree::Node) -> Vec<ParsedEntry> {
    root.children()
        .filter(|n| n.is_element() && n.tag_name().name() == "entry")
        .filter_map(|entry| {
            let thread_url = entry
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "link")
                .find(|n| {
                    matches!(n.attribute("rel"), None | Some("alternate"))
                        && n.attribute("href").is_some()
                })
                .or_else(|| {
                    entry.children().find(|n| {
                        n.is_element()
                            && n.tag_name().name() == "link"
                            && n.attribute("href").is_some()
                    })
                })
                .and_then(|n| n.attribute("href"))
                .map(str::trim)?;

            let title = child_text(entry, "title", None)
                .map_or_else(|| "Untitled".into(), |s| s.trim().to_string());

            // Atom: <content> is the full body; <summary> is always a truncated excerpt.
            // Use content if available, otherwise fall back to summary.
            let html = child_text(entry, "content", None)
                .or_else(|| child_text(entry, "summary", None))
                .unwrap_or_default();

            // Reddit: replace the thread URL with the external article link from content HTML.
            // Self-posts / discussion threads have no [link] anchor → skip them.
            let url = if thread_url.contains("reddit.com/r/") {
                normalize_article_url(&extract_reddit_url(&html)?)
            } else {
                normalize_article_url(thread_url)
            };

            let published = child_text(entry, "published", None)
                .or_else(|| child_text(entry, "updated", None))
                .and_then(|s| parse_datetime(&s))
                .unwrap_or_else(Utc::now);

            let (image, image_description) = find_image(entry, &html);
            let content = strip_html(&html);
            Some(ParsedEntry {
                url,
                title,
                content,
                published,
                image,
                image_description,
            })
        })
        .collect()
}

// JSON Feed (jsonfeed.org)
#[derive(Deserialize)]
struct JsonFeed {
    items: Vec<JsonFeedItem>,
}

#[derive(Deserialize)]
struct JsonFeedItem {
    url: Option<String>,
    title: Option<String>,
    content_html: Option<String>,
    content_text: Option<String>,
    summary: Option<String>,
    date_published: Option<String>,
    date_modified: Option<String>,
    image: Option<String>,
    banner_image: Option<String>,
}

fn parse_json_feed(bytes: &[u8]) -> Result<Vec<ParsedEntry>> {
    let feed: JsonFeed =
        serde_json::from_slice(bytes).map_err(|e| anyhow!("JSON Feed parse error: {e}"))?;

    Ok(feed
        .items
        .into_iter()
        .filter_map(|item| {
            let url = normalize_article_url(&item.url?);
            let html = join_content([item.content_html, item.content_text, item.summary]);
            let content = strip_html(&html);
            let published = item
                .date_published
                .as_deref()
                .or(item.date_modified.as_deref())
                .and_then(parse_datetime)
                .unwrap_or_else(Utc::now);
            Some(ParsedEntry {
                url,
                title: item.title.unwrap_or_else(|| "Untitled".into()),
                content,
                published,
                image: item.image.or(item.banner_image),
                image_description: None,
            })
        })
        .collect())
}

fn child_text(node: roxmltree::Node<'_, '_>, name: &str, ns: Option<&str>) -> Option<String> {
    node.children()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == name
                && ns.is_none_or(|uri| n.tag_name().namespace() == Some(uri))
        })
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
}

fn join_content<const N: usize>(parts: [Option<String>; N]) -> String {
    let joined = parts
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .unique()
        .join("\n");
    if joined.is_empty() {
        "NOT PROVIDED".into()
    } else {
        joined
    }
}

/// Try media namespace elements first, fall back to the first `<img>` in the HTML content.
fn find_image(node: roxmltree::Node, content_html: &str) -> (Option<String>, Option<String>) {
    let media = find_media_image(node);
    if media.0.is_some() {
        media
    } else {
        extract_first_img(content_html)
    }
}

fn find_media_image(node: roxmltree::Node) -> (Option<String>, Option<String>) {
    let image_url = node
        .descendants()
        .filter(roxmltree::Node::is_element)
        .find_map(|n| match (n.tag_name().namespace(), n.tag_name().name()) {
            (Some(MEDIA_NS), "thumbnail") => n.attribute("url").map(strip_query),
            (Some(MEDIA_NS), "content")
                if n.attribute("medium") == Some("image")
                    || n.attribute("type").is_some_and(|t| t.starts_with("image/")) =>
            {
                n.attribute("url").map(strip_query)
            }
            (None, "enclosure") if n.attribute("type").is_some_and(|t| t.starts_with("image/")) => {
                n.attribute("url").map(strip_query)
            }
            _ => None,
        });

    if image_url.is_none() {
        return (None, None);
    }

    // Second pass: media:description (may be a child of media:content), or thumbnail title attr
    let image_desc = node
        .descendants()
        .filter(roxmltree::Node::is_element)
        .find_map(|n| match (n.tag_name().namespace(), n.tag_name().name()) {
            (Some(MEDIA_NS), "description") => n.text().map(|s| s.trim().to_string()),
            (Some(MEDIA_NS), "thumbnail") => n.attribute("title").map(str::to_string),
            _ => None,
        });

    (image_url, image_desc)
}

/// Extract the first `<img src="...">` from an HTML string.
fn extract_first_img(html: &str) -> (Option<String>, Option<String>) {
    let Some(img_pos) = html.find("<img") else {
        return (None, None);
    };
    let Some(tag_end) = html[img_pos..].find('>').map(|p| img_pos + p) else {
        return (None, None);
    };
    let tag = &html[img_pos..=tag_end];
    let src = extract_html_attr(tag, "src").map(|s| strip_query(&decode_html_entities(&s)));
    let alt = extract_html_attr(tag, "alt").filter(|s| !s.is_empty());
    (src, alt)
}

fn extract_html_attr(tag: &str, attr: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let pattern = format!("{attr}={quote}");
        if let Some(start) = tag.find(&pattern).map(|i| i + pattern.len())
            && let Some(len) = tag[start..].find(quote)
        {
            return Some(tag[start..start + len].to_string());
        }
    }
    None
}

/// Extract the external article URL from Reddit's `[link]` anchor in feed HTML content.
/// Returns `None` for self-posts / discussion threads (no external link).
fn extract_reddit_url(html: &str) -> Option<String> {
    let link_idx = html.find(">[link]</a>")?;
    let href_start = html[..link_idx].rfind("href=\"")?;
    let url = html[href_start + 6..link_idx].trim_end_matches('"');
    Some(url.to_string())
}

fn normalize_article_url(url: &str) -> String {
    url_normalize::normalize_url(
        url,
        &NormalizeOptions {
            remove_query_parameters: RemoveQueryParameters::All,
            ..Default::default()
        },
    )
    .unwrap_or_else(|_| url.to_string())
}

fn strip_html(html: &str) -> String {
    // Insert newlines at block-level boundaries before stripping tags
    let spaced = html
        .replace("</p>", "\n")
        .replace("</div>", "\n")
        .replace("</li>", "\n")
        .replace("</h1>", "\n")
        .replace("</h2>", "\n")
        .replace("</h3>", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");

    let mut text = String::with_capacity(spaced.len());
    let mut in_tag = false;
    for ch in spaced.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }

    decode_html_entities(&text)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .join("\n")
}

fn strip_query(url: &str) -> String {
    url.split('?').next().unwrap_or(url).to_string()
}

fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp + 1..];
        let Some(semi) = rest.find(';') else {
            out.push('&');
            continue;
        };
        let entity = &rest[..semi];
        rest = &rest[semi + 1..];
        let decoded = entity.strip_prefix('#').map_or_else(
            || match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{00A0}'),
                "ndash" => Some('–'),
                "mdash" => Some('—'),
                "lsquo" => Some('\u{2018}'),
                "rsquo" => Some('\u{2019}'),
                "ldquo" => Some('\u{201C}'),
                "rdquo" => Some('\u{201D}'),
                "hellip" => Some('…'),
                "bull" => Some('•'),
                _ => None,
            },
            |num| {
                num.strip_prefix(['x', 'X'])
                    .map_or_else(|| num.parse().ok(), |hex| u32::from_str_radix(hex, 16).ok())
                    .and_then(char::from_u32)
            },
        );
        if let Some(ch) = decoded {
            out.push(ch);
        } else {
            out.push('&');
            out.push_str(entity);
            out.push(';');
        }
    }
    out.push_str(rest);
    out
}

fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.to_utc());
    }
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Some(dt.to_utc());
    }
    warn!("Could not parse datetime: {s}");
    None
}
