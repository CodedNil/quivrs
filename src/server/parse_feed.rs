use crate::server::HTTP_CLIENT;
use anyhow::{Result, anyhow};
use serde::Deserialize;
use url_normalize::{Options as NormalizeOptions, RemoveQueryParameters};

const CONTENT_NS: &str = "http://purl.org/rss/1.0/modules/content/";

/// Download and parse the feed and return a list of URLs.
pub async fn scan_feed(url_rss: &str) -> Result<Vec<String>> {
    let bytes = HTTP_CLIENT.get(url_rss).send().await?.bytes().await?;
    let first_non_ws = bytes
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .unwrap_or(0);
    let urls = if bytes.get(first_non_ws) == Some(&b'{') {
        parse_json_feed(&bytes)?
    } else {
        parse_xml_feed(&bytes)?
    };
    Ok(urls
        .into_iter()
        .filter(|url| {
            !(url.contains("bbc.co.uk/iplayer")
                || url.contains("bbc.co.uk/sounds")
                || (url.contains("reddit.com") && url.contains("/comments"))
                || url.contains("v.redd.it/")
                || url.contains("github.com")
                || url.ends_with("pdf"))
        })
        .collect())
}

fn parse_xml_feed(bytes: &[u8]) -> Result<Vec<String>> {
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

fn rss_items<'a>(nodes: impl Iterator<Item = roxmltree::Node<'a, 'a>>) -> Vec<String> {
    nodes
        .filter(|n| n.is_element() && n.tag_name().name() == "item")
        .filter_map(parse_rss_item)
        .collect()
}

fn parse_rss_item(item: roxmltree::Node) -> Option<String> {
    let thread_url = child_text(item, "link", None)
        .map(|s| s.trim().to_string())
        .or_else(|| {
            item.attributes()
                .find(|a| a.name() == "about")
                .map(|a| a.value().to_string())
        })?;

    if thread_url.contains("reddit.com/r/") {
        let html = child_text(item, "encoded", Some(CONTENT_NS))
            .or_else(|| child_text(item, "description", None))
            .unwrap_or_default();
        extract_reddit_url(&html).map(|u| normalize_article_url(&u))
    } else {
        Some(normalize_article_url(&thread_url))
    }
}

fn parse_atom(root: roxmltree::Node) -> Vec<String> {
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

            if thread_url.contains("reddit.com/r/") {
                let html = child_text(entry, "content", None)
                    .or_else(|| child_text(entry, "summary", None))
                    .unwrap_or_default();
                extract_reddit_url(&html).map(|u| normalize_article_url(&u))
            } else {
                Some(normalize_article_url(thread_url))
            }
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
}

fn parse_json_feed(bytes: &[u8]) -> Result<Vec<String>> {
    let feed: JsonFeed =
        serde_json::from_slice(bytes).map_err(|e| anyhow!("JSON Feed parse error: {e}"))?;
    Ok(feed
        .items
        .into_iter()
        .filter_map(|item| item.url.map(|u| normalize_article_url(&u)))
        .collect())
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, name: &str, ns: Option<&str>) -> Option<String> {
    node.children()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == name
                && ns.is_none_or(|uri| n.tag_name().namespace() == Some(uri))
        })
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
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
