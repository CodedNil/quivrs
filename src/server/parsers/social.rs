use crate::server::parsers::{get_cache_path, get_cached_or_fetch};
use crate::shared::ArticleSource;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;
use tokio::fs;
use tracing::info;

static SEL_META: LazyLock<Selector> = LazyLock::new(|| Selector::parse("meta").unwrap());
static SEL_SCRIPT: LazyLock<Selector> = LazyLock::new(|| Selector::parse("script").unwrap());

pub async fn fetch_social_content(url: &str) -> Result<Option<ArticleSource>> {
    if url.contains("twitter.com") || url.contains("x.com") {
        fetch_twitter_native(url).await
    } else if url.contains("bsky.app") {
        fetch_bluesky_native(url).await
    } else {
        Ok(None)
    }
}

async fn fetch_twitter_native(url: &str) -> Result<Option<ArticleSource>> {
    let html_content = get_cached_or_fetch(url).await?;
    let doc = Html::parse_document(&html_content);

    for script in doc.select(&SEL_SCRIPT) {
        let text = script.text().collect::<String>();
        if let Some(json_start) = text.find("__INITIAL_STATE__=") {
            let start = json_start + "__INITIAL_STATE__=".len();
            let end = text[start..]
                .find("};")
                .map_or_else(|| text[start..].len(), |i| i + 1);

            if let Ok(v) = serde_json::from_str::<Value>(&text[start..start + end])
                && let Some(source) = parse_twitter_initial_state(&v, url)
            {
                return Ok(Some(source));
            }
        }
    }

    let mut title = String::new();
    let mut description = String::new();
    let mut image = None;

    for el in doc.select(&SEL_META) {
        let property = el
            .value()
            .attr("property")
            .or_else(|| el.value().attr("name"));
        let content = el.value().attr("content");

        match (property, content) {
            (Some("og:title"), Some(v)) => title = v.to_string(),
            (Some("og:description"), Some(v)) => description = v.to_string(),
            (Some("og:image"), Some(v)) => image = Some(v.to_string()),
            _ => {}
        }
    }

    if description.is_empty() {
        return Ok(None);
    }

    let source = ArticleSource {
        url: url.to_string(),
        title: if title.is_empty() {
            "Tweet".to_string()
        } else {
            format!("Tweet: {title}")
        },
        summary: description
            .split('\n')
            .next()
            .unwrap_or(&description)
            .to_string(),
        content: description,
        tags: vec![],
        images: image.map(|i| vec![format!("{i}|")]).unwrap_or_default(),
        published: Utc::now(),
    };
    info!("[SOCIAL] Scraped Twitter (Meta): {:#?}", source);
    Ok(Some(source))
}

fn parse_twitter_initial_state(v: &Value, url: &str) -> Option<ArticleSource> {
    let tweet_id = url.split('/').next_back()?.split('?').next()?;
    let tweets = v.get("entities")?.get("tweets")?.get("entities")?;
    let tweet = tweets.get(tweet_id)?;

    let full_text = tweet
        .get("full_text")
        .or_else(|| tweet.get("text"))?
        .as_str()?;
    let user_id = tweet.get("user")?.as_str()?;
    let user = v
        .get("entities")?
        .get("users")?
        .get("entities")?
        .get(user_id)?;

    let name = user.get("name")?.as_str()?;
    let screen_name = user.get("screen_name")?.as_str()?;
    let user_desc = user.get("description")?.as_str().unwrap_or("");

    let summary_raw = if user_desc.is_empty() {
        full_text.to_string()
    } else {
        format!("{user_desc}: {full_text}")
    };
    let summary = summary_raw
        .split('\n')
        .next()
        .unwrap_or(&summary_raw)
        .to_string();

    let mut images = Vec::new();
    if let Some(media) = tweet
        .get("entities")
        .and_then(|e| e.get("media"))
        .and_then(|m| m.as_array())
    {
        for m in media {
            if let Some(img_url) = m.get("media_url_https").and_then(|u| u.as_str()) {
                images.push(format!("{img_url}|"));
            }
        }
    }

    let published = tweet
        .get("created_at")
        .and_then(|d| d.as_str())
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
        .map_or_else(Utc::now, |d| d.with_timezone(&Utc));

    let source = ArticleSource {
        url: url.to_string(),
        title: format!("Tweet: {name} (@{screen_name})"),
        summary,
        content: full_text.to_string(),
        tags: vec![],
        images,
        published,
    };
    info!("[SOCIAL] Scraped Twitter (State): {:#?}", source);
    Some(source)
}

async fn fetch_bluesky_native(url: &str) -> Result<Option<ArticleSource>> {
    let parts: Vec<&str> = url
        .split("/profile/")
        .last()
        .context("Invalid Bluesky URL")?
        .split("/post/")
        .collect();
    if parts.len() != 2 {
        return Ok(None);
    }
    let (handle, rkey) = (
        parts[0],
        parts[1].split('?').next().context("Invalid rkey")?,
    );

    let api_url = format!(
        "https://public.api.bsky.app/xrpc/app.bsky.feed.getPostThread?uri=at://{handle}/app.bsky.feed.post/{rkey}"
    );
    let cache_path = get_cache_path(&api_url, "json");

    let v: Value = if let Ok(bytes) = fs::read(&cache_path).await {
        serde_json::from_slice(&bytes)?
    } else {
        let resp = crate::server::HTTP_CLIENT.get(&api_url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let json_val: Value = resp.json().await?;
        if let Some(dir) = cache_path.parent()
            && fs::create_dir_all(dir).await.is_ok()
        {
            let _ = fs::write(&cache_path, serde_json::to_vec(&json_val)?).await;
        }
        json_val
    };

    let thread = v.get("thread").context("Missing thread")?;
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
    let name = author
        .get("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or(handle);

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
                let handle = p.get("author")?.get("handle")?.as_str()?;
                if handle == handle_full {
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

    let source = ArticleSource {
        url: url.to_string(),
        title: format!("Bluesky: {name} (@{handle_full})"),
        summary: main_text
            .split('\n')
            .next()
            .unwrap_or(&main_text)
            .to_string(),
        content: full_content.join("\n\n---\n\n"),
        tags: vec![],
        images,
        published,
    };
    info!("[SOCIAL] Scraped Bluesky: {:#?}", source);
    Ok(Some(source))
}
