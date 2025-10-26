use crate::llm_functions::run;
use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Duration, Utc};
use feed_rs::{model::Entry, parser};
use futures::future::join_all;
use itertools::Itertools;
use redb::{Database, ReadableTable, TableDefinition};
use regex_lite::Regex;
use reqwest::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Write,
    sync::LazyLock,
};
use tokio::fs;
use tracing::{info, warn};

pub static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
const INVIDIOUS_API_URL: &str = "https://inv.nadeko.net/api/v1";
const NITTER_API_URL: &str = "https://nitter.privacyredirect.com";

const SUMMARISE_WEBSITE: &str = "Rewrite this rss feed entry outputted as embedded HTML. Content well formatted in paragraphs, written in same the article style as the original just trimmed and concise. Include at least one image (in a figure with caption where possible, no alt text) using the original image url. The first image is used as the thumbnail and should be placed after at least the first paragraph of text. Include extra images if available at the end. Also include inline links where appropriate.";
const SUMMARISE_YOUTUBE: &str = "Rewrite this youtube video title and description as embedded HTML. The title to be a more accurate description removing clickbait questions etc while preserving the original tone, meaning and fun. The description should accurately summarise the video based on its captions, it should contain a few sentences with a few concise summary, then (only if needed) two new lines then an expansion of the summary which is still kept simple.";
const SUMMARISE_TWITTER: &str = "Rewrite this twitter post title and description as embedded HTML. The title should be a few words that accurately describe the post. The description should stay accurate to the original but clean up the formatting.";

pub const FEEDS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("feeds");
pub static DB: LazyLock<Database> = LazyLock::new(|| {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.redb".to_string());
    Database::create(database_url).unwrap()
});

#[derive(Deserialize)]
pub struct FeedConfig {
    #[serde(default)]
    source: FeedSource,
    filters: Option<Vec<String>>,
    url_rss: Option<String>,

    #[serde(default)]
    original_title: bool, // Preserve the original title
    #[serde(default)]
    original_content: bool, // Preserve the original content
}

pub type FeedConfigFile = HashMap<String, HashMap<String, FeedConfig>>;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct FeedData {
    pub title: String,
    pub url: String,
    pub url_rss: String,
    pub description: String,
    pub last_updated: DateTime<Utc>,
    pub favicon: Option<String>,
    pub icon: Option<String>,
    pub authors: Vec<String>,
    pub tags: Vec<String>,
    entries: HashMap<String, FeedEntry>,
}

impl FeedData {
    pub fn to_json_feed(&self) -> Value {
        // Sort by date_published, recent first
        let items: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.title != "INVALID")
            .sorted_unstable_by(|(_, a), (_, b)| b.published.cmp(&a.published))
            .map(|(id, entry)| entry.to_json_item(id))
            .collect();

        json!({
            "version": "https://jsonfeed.org/version/1.1",
            "title": self.title,
            "home_page_url": self.url,
            "feed_url": self.url_rss,
            "description": self.description,
            "icon": self.icon,
            "favicon": self.favicon,
            "items": items,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct FeedEntry {
    title: String,
    link: String,
    description: String,
    published: DateTime<Utc>,
    image: Option<String>,
    authors: Vec<String>,
    tags: Vec<String>,
}

impl FeedEntry {
    fn to_json_item(&self, id: &str) -> Value {
        json!({
            "id": id,
            "url": self.link,
            "title": self.title,
            "content_html": self.description,
            "image": self.image,
            "banner_image": self.image,
            "date_published": self.published,
            "authors": self.authors.iter().map(|name| json!({"name": name})).collect::<Vec<_>>(),
            "tags": self.tags
        })
    }

    fn invalid() -> Self {
        Self {
            title: "INVALID".to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
enum FeedSource {
    #[default]
    Website,
    Youtube,
    Twitter,
    Reddit,
}

/// Refreshes all feeds.
pub async fn refresh_all_feeds() -> Result<()> {
    // Load config file
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.json".to_string());
    let config_str = fs::read_to_string(&config_path).await?;
    let config_file: FeedConfigFile = serde_json::from_str(&config_str)
        .map_err(|e| anyhow!("Failed to read {config_path}: {e}"))?;

    // Go through all the feeds and refresh them
    let write_txn = DB.begin_write()?;
    let mut collected_feeds = HashMap::new();
    let mut feed_keys = HashSet::new();
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        let mut tasks = Vec::new();

        for feeds in config_file.values() {
            for (feed_id, config) in feeds {
                feed_keys.insert(feed_id.clone());

                // Get stored feed data
                let mut feed: FeedData = table
                    .get(feed_id.as_str())?
                    .and_then(|g| postcard::from_bytes(g.value()).ok())
                    .unwrap_or_default();

                tasks.push(async move {
                    // Override with the config
                    feed.url_rss = if let Some(url_rss) = &config.url_rss {
                        url_rss.clone()
                    } else {
                        match config.source {
                            FeedSource::Youtube => format!(
                                "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                                resolve_youtube_channel_id(feed_id).await?
                            ),
                            FeedSource::Twitter => format!("{NITTER_API_URL}/{feed_id}/rss"),
                            _ => feed.url_rss,
                        }
                    };

                    // Refresh the feed data
                    refresh_feed(feed_id, config, &mut feed).await?;

                    // Override with the config
                    feed.url = match config.source {
                        FeedSource::Youtube => format!("https://www.youtube.com/@{feed_id}"),
                        FeedSource::Twitter => format!("https://x.com/{feed_id}"),
                        _ => feed.url,
                    };

                    Ok::<_, anyhow::Error>((feed_id, feed))
                });
            }
        }

        // Wait for all tasks to complete
        for (feed_id, feed) in join_all(tasks).await.iter().flatten() {
            table.insert(feed_id.as_str(), postcard::to_allocvec(&feed)?.as_slice())?;
            collected_feeds.insert((*feed_id).to_string(), feed.clone());
        }

        // Remove feeds that are no longer in the config
        table.retain(|key, _| feed_keys.contains(key))?;
    }
    write_txn.commit()?;

    #[cfg(not(debug_assertions))]
    if let Err(e) = crate::miniflux::update_feeds(&config_file, collected_feeds).await {
        tracing::error!("Failed to update miniflux feeds: {e}");
    }

    Ok(())
}

/// Refreshes a single feed and updates the cache.
async fn refresh_feed(feed_id: &str, config: &FeedConfig, feed: &mut FeedData) -> Result<()> {
    // Fetch and parse the remote feed
    let content = HTTP_CLIENT.get(&feed.url_rss).send().await?.bytes().await?;
    let fetched = parser::parse(content.as_ref())?;

    if let Some(title) = fetched.title {
        feed.title = title.content;
    }
    if let Some(url) = fetched
        .links
        .iter()
        .find(|l| l.media_type.as_deref() == Some("text/html"))
        .or_else(|| fetched.links.first())
    {
        feed.url.clone_from(&url.href);
    }
    if let Some(description) = fetched.description {
        feed.description = description.content;
    }
    if let Some(last_updated) = fetched.updated {
        feed.last_updated = last_updated;
    }
    feed.icon = fetched.logo.map(|icon| icon.uri);
    feed.favicon = fetched.icon.map(|icon| icon.uri);
    feed.authors = fetched.authors.iter().map(|a| a.name.clone()).collect();
    feed.tags = fetched.categories.iter().map(|c| c.term.clone()).collect();

    // Process all entries concurrently
    for result in join_all(
        fetched
            .entries
            .iter()
            .filter(|entry| !feed.entries.contains_key(&entry.id))
            .map(|entry| build_item(feed_id, config, entry, &fetched.entries)),
    )
    .await
    {
        match result {
            Ok((entry_id, entry)) => {
                feed.entries.insert(entry_id, entry);
            }
            Err(err) => warn!(feed_id = %feed_id, "Failed to build entry: {err:#}"),
        }
    }

    Ok(())
}

/// Builds a single RSS item, checking for existing items and optionally summarising content.
async fn build_item(
    feed_id: &str,
    config: &FeedConfig,
    entry: &Entry,
    other_entries: &[Entry],
) -> Result<(String, FeedEntry)> {
    let mut link = entry
        .links
        .first()
        .ok_or_else(|| anyhow!("Entry has no link."))?
        .href
        .clone();

    let mut title = entry
        .title
        .as_ref()
        .map_or_else(|| "NOT PROVIDED".to_string(), |t| t.content.clone());
    let mut description = entry
        .content
        .as_ref()
        .and_then(|c| c.body.clone())
        .or_else(|| entry.summary.as_ref().map(|s| s.content.clone()))
        .unwrap_or_else(|| "NOT PROVIDED".to_string());
    let published = entry.published.unwrap_or_else(Utc::now);

    // If twitter, swap the link back to twitter, and reject replies
    if matches!(config.source, FeedSource::Twitter) {
        if title.starts_with("R to @") {
            return Ok((entry.id.clone(), FeedEntry::invalid()));
        }
        if let Some(end) = link.strip_prefix(NITTER_API_URL) {
            link = format!("https://x.com{end}");
        }

        // Grab other tweets that were posted within 5 minutes and are replies to the same user, append the descriptions to build the thread
        let thread: Vec<(String, DateTime<Utc>)> = other_entries
            .iter()
            .filter_map(|other| {
                let other_title = other.title.as_ref()?.content.clone();
                let other_time = other.published?;
                (other_title.starts_with(&format!("R to @{feed_id}"))
                    && other_time > published
                    && (other_time - published) < Duration::minutes(5))
                .then_some((other_title, other_time))
            })
            .sorted_unstable_by_key(|(_, pub_time)| *pub_time)
            .collect();

        // Sort by published time (earliest first), then push into the description
        if !thread.is_empty() {
            description.push_str("\n\n");
            for (i, (body, _)) in thread.iter().enumerate() {
                writeln!(
                    description,
                    "Reply post [{}/{}] in thread : {body}",
                    i + 1,
                    thread.len()
                )?;
            }
        }
    }

    let summarised = match config.source {
        FeedSource::Youtube => {
            if let Some(media_description) = entry.media.iter().find_map(|m| m.description.as_ref())
            {
                description.clone_from(&media_description.content);
            }

            // Get the video ID from the link
            let Some(video_id) = link
                .split_once("v=")
                .and_then(|(_, rest)| rest.split('&').next())
            else {
                // Invalid youtube link, probably a youtube short
                return Ok((entry.id.clone(), FeedEntry::invalid()));
            };

            // Load youtube captions
            let caption_link = format!("{INVIDIOUS_API_URL}/captions/{video_id}?label=English");
            let response = HTTP_CLIENT.get(caption_link).send().await?;
            let captions = response.text().await.unwrap_or_default();

            if !config.original_title || !config.original_content || config.filters.is_some() {
                summarise_content(
                    &link,
                    config,
                    &title,
                    &description,
                    SUMMARISE_YOUTUBE,
                    "Videos captions:",
                    captions
                        .lines()
                        .filter(|line| !line.contains("-->") && !line.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .replace("  ", " ")
                        .trim(),
                )
                .await?
            } else {
                SummariseOutput {
                    title: title.clone(),
                    content: description.clone(),
                    image: None,
                    included: true,
                }
            }
        }
        FeedSource::Twitter => {
            let mut summarised = summarise_content(
                &link,
                config,
                &title,
                &description,
                SUMMARISE_TWITTER,
                "",
                "",
            )
            .await?;

            // Swap out the nitter images with original twitter ones
            let replace_nitter_url = |s: String| {
                s.replace(&format!("{NITTER_API_URL}/pic"), "https://pbs.twimg.com")
                    .replace("%2F", "/")
            };
            summarised.image = summarised.image.map(replace_nitter_url);
            summarised.content = replace_nitter_url(summarised.content);

            summarised
        }
        _ => {
            // Fetch the site and parse the html into markdown
            let original_content = HTTP_CLIENT.get(&link).send().await?.text().await?;

            summarise_content(
                &link,
                config,
                &title,
                &description,
                SUMMARISE_WEBSITE,
                "Original content:",
                &html2text::from_read(original_content.as_bytes(), 20)?,
            )
            .await?
        }
    };

    if !config.original_title {
        title = summarised.title;
    }
    if !config.original_content {
        description = summarised.content;
    }

    if config.filters.is_some() && !summarised.included {
        info!("FILTERED: {link}");
        return Ok((entry.id.clone(), FeedEntry::invalid()));
    }

    // Use media thumbnail if that exists, else summarised image
    let image = entry
        .media
        .first()
        .and_then(|m| m.thumbnails.first().map(|t| t.image.uri.clone()))
        .or(summarised.image);

    Ok((
        entry.id.clone(),
        FeedEntry {
            title,
            link,
            description,
            published,
            authors: entry.authors.iter().map(|p| p.name.clone()).collect(),
            tags: entry.categories.iter().map(|c| c.term.clone()).collect(),
            image,
        },
    ))
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct SummariseOutput {
    /// The rewritten title, kept concise and descriptive.
    title: String,
    /// Summarised content, well written and engaging.
    content: String,
    /// Key image url, to represent the feed as a thumbnail, if the feed doesn't provide leave this empty.
    image: Option<String>,
    /// Whether this entry should be included, false for filtered out.
    included: bool,
}

/// Summarises content for a feed entry.
async fn summarise_content(
    link: &str,
    config: &FeedConfig,
    title: &str,
    description: &str,
    prompt: &str,
    content_key: &str,
    original_content: &str,
) -> Result<SummariseOutput> {
    let mut context = vec![
        format!(
            "Rules: do NOT include ```json at the start, directly output as json with the outer structure being {{key: value}}"
        ),
        format!("Original title: {title}"),
        format!("Original description: {description}"),
    ];
    if !content_key.is_empty() && !original_content.is_empty() {
        context.push(format!("{content_key} {original_content}"));
    }
    if let Some(filters) = config
        .filters
        .as_ref()
        .filter(|filters| !filters.is_empty())
    {
        context.push(format!(
            "Users requested filters, do not include posts including any of these '{}'",
            filters.join(";")
        ));
    }

    run::<SummariseOutput>(context, prompt)
        .await
        .map_err(|err| {
            warn!(link = %link, "Summarisation failed: {err:#}");
            anyhow!("Summarisation failed: {err}")
        })
}

/// Resolves a `YouTube` channel ID from a given URL or by fetching the page content.
async fn resolve_youtube_channel_id(channel_name: &str) -> Result<String> {
    let body = HTTP_CLIENT
        .get(format!("https://www.youtube.com/@{channel_name}"))
        .send()
        .await?
        .text()
        .await?;
    if let Some(caps) =
        Regex::new(r#"<link\s+rel=["']?canonical["']?\s+href=["']([^"']+)["']"#)?.captures(&body)
        && let Some(channel_id) = caps[1].rsplit('/').find(|segment| !segment.is_empty())
    {
        return Ok(channel_id.to_string());
    }
    bail!("No channel ID found for `{channel_name}`")
}
