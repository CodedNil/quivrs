mod miniflux;

use anyhow::{Result, anyhow, bail};
use futures::future::join_all;
use miniflux::{FeedTarget, update_feeds};
use regex_lite::Regex;
use reqwest::Client;
use std::{collections::HashMap, env, sync::LazyLock};
use tokio::fs;
use tracing::warn;
use tracing_subscriber::EnvFilter;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

type FeedConfigFile = HashMap<String, Vec<String>>;

enum FeedConfig {
    Direct { name: String, url: String },
    Youtube { channel: String },
    Twitter { handle: String },
    Bluesky { handle: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    sync_feeds().await
}

async fn sync_feeds() -> Result<()> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.json".to_string());
    let config_str = fs::read_to_string(&config_path).await?;
    let config_file: FeedConfigFile = serde_json::from_str(&config_str)
        .map_err(|err| anyhow!("Failed to read {config_path}: {err}"))?;

    let targets = resolve_feed_targets(&config_file).await;
    update_feeds(config_file.keys(), targets).await
}

async fn resolve_feed_targets(config_file: &FeedConfigFile) -> Vec<FeedTarget> {
    let tasks = config_file
        .iter()
        .flat_map(|(category, feeds)| feeds.iter().map(move |feed| (category, feed)))
        .map(|(category, feed)| async move {
            match parse_feed_config(feed) {
                Ok(config) => match resolve_feed_target(category, config).await {
                    Ok(target) => Some(target),
                    Err(err) => {
                        warn!(feed = %feed, "Skipping feed: {err:#}");
                        None
                    }
                },
                Err(err) => {
                    warn!(feed = %feed, "Skipping feed: {err:#}");
                    None
                }
            }
        });

    join_all(tasks).await.into_iter().flatten().collect()
}

fn parse_feed_config(feed: &str) -> Result<FeedConfig> {
    let (kind_or_name, value) = feed
        .split_once('/')
        .ok_or_else(|| anyhow!("feed must contain a slash"))?;

    match kind_or_name.to_ascii_lowercase().as_str() {
        "youtube" => Ok(FeedConfig::Youtube {
            channel: value.to_string(),
        }),
        "twitter" => Ok(FeedConfig::Twitter {
            handle: value.to_string(),
        }),
        "bluesky" => Ok(FeedConfig::Bluesky {
            handle: value.to_string(),
        }),
        _ => Ok(FeedConfig::Direct {
            name: kind_or_name.to_string(),
            url: value.to_string(),
        }),
    }
}

async fn resolve_feed_target(category: &str, config: FeedConfig) -> Result<FeedTarget> {
    match config {
        FeedConfig::Direct { name, url } => Ok(FeedTarget {
            name,
            category: category.to_string(),
            feed_url: url,
            site_url: None,
        }),
        FeedConfig::Youtube { channel } => {
            let channel_id = resolve_youtube_channel_id(&channel).await?;
            Ok(FeedTarget {
                name: channel.clone(),
                category: category.to_string(),
                feed_url: format!(
                    "https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}"
                ),
                site_url: Some(format!("https://www.youtube.com/@{channel}")),
            })
        }
        FeedConfig::Twitter { handle } => Ok(FeedTarget {
            name: handle.clone(),
            category: category.to_string(),
            feed_url: format!(
                "https://fxtwitter.com/{}/feed.atom.xml",
                normalise_social_handle(&handle)
            ),
            site_url: Some(format!(
                "https://x.com/{}",
                normalise_social_handle(&handle)
            )),
        }),
        FeedConfig::Bluesky { handle } => Ok(FeedTarget {
            name: handle.clone(),
            category: category.to_string(),
            feed_url: format!(
                "https://fxbsky.app/profile/{}/feed.atom.xml",
                normalise_social_handle(&handle)
            ),
            site_url: Some(format!(
                "https://bsky.app/profile/{}",
                normalise_social_handle(&handle)
            )),
        }),
    }
}

fn normalise_social_handle(handle: &str) -> &str {
    handle.trim_start_matches('@')
}

async fn resolve_youtube_channel_id(channel_name: &str) -> Result<String> {
    if channel_name.starts_with("UC") {
        return Ok(channel_name.to_string());
    }

    let body = HTTP_CLIENT
        .get(format!("https://www.youtube.com/@{channel_name}"))
        .send()
        .await?
        .error_for_status()
        .map_err(|err| anyhow!("YouTube handle page request failed for `{channel_name}`: {err}"))?
        .text()
        .await?;

    if let Some(channel_id) = extract_youtube_channel_id(&body)? {
        return Ok(channel_id);
    }

    bail!("No channel ID found for `{channel_name}`")
}

fn extract_youtube_channel_id(body: &str) -> Result<Option<String>> {
    for pattern in [
        r"feeds/videos\.xml\?channel_id=(UC[a-zA-Z0-9_-]+)",
        r#""browseId":"(UC[^"]+)""#,
        r#""channelId":"(UC[^"]+)""#,
        r#"<meta\s+itemprop=["']channelId["']\s+content=["']([^"']+)["']"#,
        r#"<link\s+rel=["']?canonical["']?\s+href=["'][^"']*/(UC[^/"']+)["']"#,
    ] {
        if let Some(caps) = Regex::new(pattern)?.captures(body) {
            return Ok(Some(caps[1].to_string()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::extract_youtube_channel_id;

    #[test]
    fn extracts_youtube_channel_id_from_feed_link() {
        let body = r#"<link rel="alternate" type="application/rss+xml" href="https://www.youtube.com/feeds/videos.xml?channel_id=UCy0tKL1T7wFoYcxCe0xjN6Q">"#;

        assert_eq!(
            extract_youtube_channel_id(body).unwrap().as_deref(),
            Some("UCy0tKL1T7wFoYcxCe0xjN6Q")
        );
    }

    #[test]
    fn extracts_youtube_channel_id_from_browse_id() {
        let body = r#""browseEndpoint":{"browseId":"UCy0tKL1T7wFoYcxCe0xjN6Q"}"#;

        assert_eq!(
            extract_youtube_channel_id(body).unwrap().as_deref(),
            Some("UCy0tKL1T7wFoYcxCe0xjN6Q")
        );
    }
}
