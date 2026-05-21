use crate::server::HTTP_CLIENT;
use anyhow::{Result, anyhow};
use serde::Deserialize;
use tracing::warn;
use url_normalize::{Options as NormalizeOptions, QueryFilter, RemoveQueryParameters};

/// Download and parse the feed and return a list of URLs.
pub async fn scan_feed(url_rss: &str) -> Result<Vec<String>> {
    let urls = if url_rss.contains("reddit.com/r/") {
        let json_url = url_rss
            .trim_end_matches('/')
            .trim_end_matches(".rss")
            .trim_end_matches(".json")
            .to_string()
            + ".json";
        let bytes = HTTP_CLIENT.get(&json_url).send().await?.bytes().await?;
        parse_reddit_json(&bytes)?
    } else {
        let bytes = HTTP_CLIENT.get(url_rss).send().await?.bytes().await?;
        let first_non_ws = bytes
            .iter()
            .position(|&b| !b.is_ascii_whitespace())
            .unwrap_or(0);
        if bytes.get(first_non_ws) == Some(&b'{') {
            parse_json_feed(&bytes)?
        } else {
            match parse_xml_feed(&bytes) {
                Ok(urls) => urls,
                Err(e) => {
                    warn!("Failed to parse XML feed {url_rss}: {e}");
                    vec![]
                }
            }
        }
    };
    Ok(urls
        .into_iter()
        .filter(|url| {
            !(url.contains("bbc.co.uk/iplayer")
                || url.contains("bbc.co.uk/sounds")
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

    Some(normalize_article_url(&thread_url))
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

            Some(normalize_article_url(thread_url))
        })
        .collect()
}

// Reddit JSON API (reddit.com/r/sub.json)
#[derive(Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Deserialize)]
struct RedditPost {
    url: String,
    score: i32,
    upvote_ratio: f32,
}

fn parse_reddit_json(bytes: &[u8]) -> Result<Vec<String>> {
    let listing: RedditListing = serde_json::from_slice(bytes).map_err(|e| {
        let preview = std::str::from_utf8(&bytes[..bytes.len().min(200)])
            .unwrap_or("<invalid utf-8>")
            .trim();
        anyhow!("Reddit JSON parse error: {e} — response: {preview}")
    })?;
    Ok(listing
        .data
        .children
        .into_iter()
        .map(|c| c.data)
        .filter(|p| p.score > 10 && p.upvote_ratio > 0.8)
        .map(|p| p.url)
        .filter(|url| !url.contains("reddit.com") && !url.contains("redd.it"))
        .map(|url| normalize_article_url(&url))
        .collect())
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

/// Only strip known tracking parameters
fn normalize_article_url(url: &str) -> String {
    const TRACKING_PARAMS: &[&str] = &[
        "ref", "source", "fbclid", "gclid", "msclkid", "yclid", "igshid",
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
