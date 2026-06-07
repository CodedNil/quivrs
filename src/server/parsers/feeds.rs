use crate::server::{
    HTTP_CLIENT,
    parsers::{is_base_url, normalize_article_url, usable_article_url},
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::warn;

/// Download and parse the feed and return a list of URLs.
pub async fn scan_feed(url_rss: &str) -> Result<Vec<String>> {
    let is_twitter = url_rss.contains("twitter.com") || url_rss.contains("x.com");
    let is_bluesky = url_rss.contains("bsky.app/profile/");
    if (is_twitter || is_bluesky) && !url_rss.contains("/status/") && !url_rss.contains("/post/") {
        return super::social::scan_social_profile(url_rss).await;
    }

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
    Ok(urls)
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
    if node_is_too_old(item, &["pubDate", "date"]) {
        return None;
    }

    let thread_url = child_text(item, "link", None)
        .map(|s| s.trim().to_string())
        .or_else(|| {
            item.attributes()
                .find(|a| a.name() == "about")
                .map(|a| a.value().to_string())
        })?;

    usable_article_url(&thread_url)
}

fn parse_atom(root: roxmltree::Node) -> Vec<String> {
    root.children()
        .filter(|n| n.is_element() && n.tag_name().name() == "entry")
        .filter_map(|entry| {
            if node_is_too_old(entry, &["published", "updated"]) {
                return None;
            }

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

            usable_article_url(thread_url)
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
    created_utc: Option<f64>,
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
        .filter(|p| {
            !p.created_utc
                .and_then(|timestamp| DateTime::from_timestamp(timestamp as i64, 0))
                .is_some_and(super::is_article_too_old)
        })
        .filter(|p| p.score > 10 && p.upvote_ratio > 0.8)
        .map(|p| p.url)
        .filter(|url| !url.contains("reddit.com") && !url.contains("redd.it"))
        .map(|url| normalize_article_url(&url))
        .filter(|url| !is_base_url(url))
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
    date_published: Option<String>,
    date_modified: Option<String>,
}

fn parse_json_feed(bytes: &[u8]) -> Result<Vec<String>> {
    let feed: JsonFeed =
        serde_json::from_slice(bytes).map_err(|e| anyhow!("JSON Feed parse error: {e}"))?;
    Ok(feed
        .items
        .into_iter()
        .filter_map(|item| {
            if [&item.date_published, &item.date_modified]
                .into_iter()
                .flatten()
                .find_map(|date| parse_feed_date(date))
                .is_some_and(super::is_article_too_old)
            {
                return None;
            }

            item.url.and_then(|url| usable_article_url(&url))
        })
        .collect())
}

fn node_is_too_old(node: roxmltree::Node, date_names: &[&str]) -> bool {
    date_names
        .iter()
        .filter_map(|name| child_text(node, name, None))
        .find_map(|date| parse_feed_date(&date))
        .is_some_and(super::is_article_too_old)
}

fn parse_feed_date(date: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date)
        .or_else(|_| DateTime::parse_from_rfc2822(date))
        .ok()
        .map(|date| date.with_timezone(&Utc))
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
