use crate::server::{HTTP_CLIENT, parsers::get_cached_or_fetch_ext};
use crate::shared::ArticleSource;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::info;

const FXTWITTER_API: &str = "https://api.fxtwitter.com/2";

fn build_summary(text: &str) -> String {
    const MAX_LEN: usize = 200;
    const MIN_LEN: usize = 80; // prefer clean line breaks only once we have this much

    let mut result = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let sep = usize::from(!result.is_empty());
        if result.len() + sep + line.len() > MAX_LEN {
            if result.len() >= MIN_LEN {
                break; // enough content — stop at clean line boundary
            }
            // not enough yet — include this line truncated
            if !result.is_empty() {
                result.push(' ');
            }
            let available = MAX_LEN - result.len();
            let cutoff = (0..=available.min(line.len()))
                .rev()
                .find(|&i| line.is_char_boundary(i))
                .unwrap_or(0);
            let end = line[..cutoff]
                .rfind(' ')
                .filter(|&i| cutoff - i <= 10)
                .unwrap_or(cutoff);
            result.push_str(&line[..end]);
            break;
        }
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(line);
    }
    result
}

fn short_title(text: &str, handle: &str) -> String {
    const MAX: usize = 80;
    let snippet = if text.len() <= MAX {
        text
    } else {
        text[..MAX].rfind(' ').map_or(&text[..MAX], |i| &text[..i])
    };
    format!("{snippet} @{handle}")
}

/// Fetch the content of an individual social post
pub async fn fetch_social_content(url: &str) -> Result<Option<ArticleSource>> {
    if url.contains("twitter.com") || url.contains("x.com") {
        fetch_twitter_native(url).await
    } else if url.contains("bsky.app") {
        fetch_bluesky_native(url).await
    } else {
        Ok(None)
    }
}

/// Read recent posts from a profile
pub async fn scan_social_profile(url: &str) -> Result<Vec<String>> {
    if url.contains("bsky.app/profile/") {
        scan_bluesky_profile(url).await
    } else {
        scan_twitter_profile(url).await
    }
}

/// Read recent twitter posts from a profile
async fn scan_twitter_profile(url: &str) -> Result<Vec<String>> {
    let username = url
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .and_then(|s| s.split('?').next())
        .context("Invalid Twitter profile URL")?;

    let api_url = format!("{FXTWITTER_API}/profile/{username}/statuses");
    let resp = HTTP_CLIENT.get(&api_url).send().await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let v: Value = resp.json().await?;

    let urls: Vec<String> = v
        .get("results")
        .and_then(|r| r.as_array())
        .map(|results| {
            results
                .iter()
                .filter_map(|s| {
                    // Skip thread continuations and reposts
                    if ["replying_to", "reposted_by"]
                        .iter()
                        .any(|k| s.get(k).is_some_and(|v| !v.is_null()))
                    {
                        return None;
                    }
                    let id = s.get("id")?.as_str()?;
                    Some(format!("https://x.com/{username}/status/{id}"))
                })
                .collect()
        })
        .unwrap_or_default();

    info!(
        "[SOCIAL] Found {} tweets from @{username} via fxtwitter",
        urls.len()
    );
    Ok(urls)
}

/// Parse an individual twitter post
async fn fetch_twitter_native(url: &str) -> Result<Option<ArticleSource>> {
    let tweet_id = url
        .split('/')
        .next_back()
        .and_then(|s| s.split('?').next())
        .context("Invalid tweet URL")?;

    let api_url = format!("{FXTWITTER_API}/thread/{tweet_id}");
    let json = get_cached_or_fetch_ext(&api_url, "json").await?;
    let v: Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    let author = v.get("author").context("Missing author")?;
    let screen_name = author
        .get("screen_name")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    let thread = v
        .get("thread")
        .and_then(|t| t.as_array())
        .context("Missing thread")?;

    let mut content_parts: Vec<String> = Vec::new();
    let mut images: Vec<String> = Vec::new();

    for tweet in thread {
        if let Some(text) = tweet.get("text").and_then(|t| t.as_str()) {
            content_parts.push(text.to_string());
        }
        if let Some(photos) = tweet
            .get("media")
            .and_then(|m| m.get("photos"))
            .and_then(|p| p.as_array())
        {
            for photo in photos {
                if let Some(photo_url) = photo.get("url").and_then(|u| u.as_str()) {
                    images.push(format!("{photo_url}|"));
                }
            }
        }
    }

    if content_parts.is_empty() {
        return Ok(None);
    }

    let published = v
        .get("status")
        .and_then(|s| s.get("created_at"))
        .and_then(|d| d.as_str())
        .and_then(|d| DateTime::parse_from_str(d, "%a %b %d %H:%M:%S %z %Y").ok())
        .map_or_else(Utc::now, |d| d.with_timezone(&Utc));

    let content = content_parts.join("\n\n---\n\n");
    let first_text = content_parts.first().map_or("", String::as_str);
    let summary = build_summary(first_text);
    let title_line = first_text.lines().next().unwrap_or("");

    Ok(Some(ArticleSource {
        url: url.to_string(),
        title: short_title(title_line, screen_name),
        summary,
        content,
        tags: vec![],
        images,
        published,
        source: screen_name.to_string(),
    }))
}

/// Read recent bluesky posts from a profile
async fn scan_bluesky_profile(url: &str) -> Result<Vec<String>> {
    let handle = url
        .split("/profile/")
        .nth(1)
        .context("Invalid Bluesky profile URL")?
        .trim_end_matches('/');

    let api_url = format!(
        "https://public.api.bsky.app/xrpc/app.bsky.feed.getAuthorFeed?actor={handle}&filter=posts_no_replies&limit=30"
    );

    let resp = HTTP_CLIENT.get(&api_url).send().await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let v: Value = resp.json().await?;

    let Some(feed) = v.get("feed").and_then(|f| f.as_array()) else {
        return Ok(vec![]);
    };

    let urls: Vec<String> = feed
        .iter()
        .filter_map(|item| {
            if item.get("reason").is_some() {
                return None;
            }
            let post = item.get("post")?;
            let uri = post.get("uri")?.as_str()?;
            let rkey = uri.split('/').next_back()?;
            let post_handle = post.get("author")?.get("handle")?.as_str()?;
            Some(format!(
                "https://bsky.app/profile/{post_handle}/post/{rkey}"
            ))
        })
        .collect();

    info!(
        "[SOCIAL] Found {} posts from Bluesky profile @{}",
        urls.len(),
        handle
    );
    Ok(urls)
}

/// Parse an individual bluesky post
async fn fetch_bluesky_native(url: &str) -> Result<Option<ArticleSource>> {
    let after_profile = url
        .split("/profile/")
        .nth(1)
        .context("Invalid Bluesky URL")?;
    let Some((handle, rest)) = after_profile.split_once("/post/") else {
        return Ok(None);
    };
    let rkey = rest.split('?').next().context("Invalid rkey")?;

    let api_url = format!(
        "https://public.api.bsky.app/xrpc/app.bsky.feed.getPostThread?uri=at://{handle}/app.bsky.feed.post/{rkey}"
    );

    let text = get_cached_or_fetch_ext(&api_url, "json").await?;
    let v: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    let Some(thread) = v.get("thread") else {
        return Ok(None);
    };
    let post = thread.get("post").context("Missing post")?;
    let author = post.get("author").context("Missing author")?;
    let record = post.get("record").context("Missing record")?;

    let main_text = record
        .get("text")
        .and_then(|t| t.as_str())
        .context("Missing text")?
        .to_string();
    let handle_full = author
        .get("handle")
        .and_then(|h| h.as_str())
        .context("Missing handle")?;

    let mut full_content = Vec::new();
    let mut current_parent = thread.get("parent");
    while let Some(parent_wrapper) = current_parent {
        if let Some(p_post) = parent_wrapper.get("post") {
            let author_match = p_post
                .get("author")
                .and_then(|a| a.get("handle"))
                .and_then(|h| h.as_str())
                == Some(handle_full);

            if author_match
                && let Some(text) = p_post
                    .get("record")
                    .and_then(|r| r.get("text"))
                    .and_then(|t| t.as_str())
            {
                full_content.push(text.to_string());
            }
            current_parent = parent_wrapper.get("parent");
        } else {
            break;
        }
    }
    full_content.reverse();
    full_content.push(main_text.clone());

    if let Some(replies) = thread.get("replies").and_then(|r| r.as_array()) {
        let mut author_replies: Vec<_> = replies
            .iter()
            .filter_map(|r| {
                let p = r.get("post")?;
                let reply_handle = p.get("author")?.get("handle")?.as_str()?;
                if reply_handle == handle_full {
                    let rec = p.get("record")?;
                    Some((rec.get("createdAt")?.as_str()?, rec.get("text")?.as_str()?))
                } else {
                    None
                }
            })
            .collect();
        author_replies.sort_by_key(|k| k.0);
        full_content.extend(author_replies.into_iter().map(|(_, t)| t.to_string()));
    }

    let mut images = Vec::new();
    if let Some(embed) = post.get("embed")
        && let Some(imgs) = embed.get("images").and_then(|i| i.as_array())
    {
        for img in imgs {
            if let Some(thumb) = img.get("thumb").and_then(|t| t.as_str()) {
                images.push(format!("{thumb}|"));
            }
        }
    }

    let published = record
        .get("createdAt")
        .and_then(|d| d.as_str())
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
        .map_or_else(Utc::now, |d| d.with_timezone(&Utc));

    let summary = build_summary(&main_text);
    let title_line = main_text.lines().next().unwrap_or("");
    Ok(Some(ArticleSource {
        url: url.to_string(),
        title: short_title(title_line, handle_full),
        summary,
        content: full_content.join("\n\n---\n\n"),
        tags: vec![],
        images,
        published,
        source: handle_full.to_string(),
    }))
}
