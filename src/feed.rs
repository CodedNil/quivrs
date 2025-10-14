use crate::llm_functions::run;
use anyhow::{Context, Result, anyhow, bail};
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bincode::{Decode, Encode, config};
use chrono::{DateTime, Utc};
use feed_rs::{model::Entry, parser};
use futures::future::join_all;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use reqwest::{Client, header::CONTENT_TYPE};
use rss::{ChannelBuilder, Guid, ItemBuilder};
use schemars::JsonSchema;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::LazyLock,
};
use tokio::fs;
use tracing::{info, warn};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
const INVIDIOUS_API_URL: &str = "https://inv.nadeko.net/api/v1";

pub const FEEDS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("feeds");
pub static DB: LazyLock<Database> = LazyLock::new(|| {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.redb".to_string());
    Database::create(database_url).unwrap()
});

#[derive(Encode, Decode, Deserialize)]
pub struct FeedData {
    pub source: FeedSource,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub clean_title: bool,
    #[serde(default)]
    pub summarise_content: bool,

    pub url_rss: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub entries: HashMap<String, FeedEntry>,
}

impl FeedData {
    pub fn to_rss_channel(&self) -> rss::Channel {
        let mut entries = self
            .entries
            .values()
            .map(FeedEntry::to_rss_item)
            .collect::<Vec<rss::Item>>();

        // Sort by date published, recent first
        entries.sort_by(|a, b| {
            let key_a = a
                .pub_date()
                .and_then(|s| DateTime::parse_from_rfc2822(s).ok());
            let key_b = b
                .pub_date()
                .and_then(|s| DateTime::parse_from_rfc2822(s).ok());
            key_b.cmp(&key_a)
        });

        ChannelBuilder::default()
            .title(self.title.clone())
            .link(self.url.clone())
            .description(self.description.clone())
            .items(entries)
            .last_build_date(Some(self.last_updated.clone()))
            .build()
    }
}

#[derive(Encode, Decode, Deserialize)]
pub struct FeedEntry {
    pub id: String,
    pub title: String,
    pub link: String,
    pub description: String,
    pub published: String,
    pub author: String,
}

impl FeedEntry {
    pub fn to_rss_item(&self) -> rss::Item {
        ItemBuilder::default()
            .title(Some(self.title.clone()))
            .link(Some(self.link.clone()))
            .description(Some(self.description.clone()))
            .guid(Guid {
                value: self.id.clone(),
                permalink: true,
            })
            .pub_date(Some(self.published.clone()))
            .build()
    }
}

#[derive(Encode, Decode, Deserialize)]
pub enum FeedSource {
    Website,
    Youtube,
    Twitter,
    Reddit,
}

#[derive(Deserialize)]
pub struct Config {
    feeds: HashMap<String, FeedData>,
}

/// Helper function to encode `FeedData`
fn encode_feed_data(feed_data: &FeedData) -> Result<Vec<u8>> {
    Ok(bincode::encode_to_vec(feed_data, config::standard())?)
}

/// Helper function to decode `FeedData`
fn decode_feed_data(bytes: &[u8]) -> Result<FeedData> {
    Ok(bincode::decode_from_slice(bytes, config::standard())?.0)
}

/// Resolves a `YouTube` channel ID from a given URL or by fetching the page content.
async fn resolve_youtube_channel_id(url: &str) -> Result<String> {
    let body = HTTP_CLIENT.get(url).send().await?.text().await?;
    let document = Html::parse_document(&body);
    let selector = Selector::parse("link[rel=\"canonical\"]").unwrap();
    if let Some(element) = document.select(&selector).next()
        && let Some(href) = element.value().attr("href")
        && let Some(channel_id) = href.split('/').next_back()
    {
        info!("Found channel ID for {url}: {channel_id}",);
        return Ok(channel_id.to_string());
    }

    bail!("No channel ID found for `{url}`")
}

/// Ensure the `redb` database is ready for use.
pub async fn init_storage() -> Result<()> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.toml".to_string());
    let config: Config = toml::from_str(&fs::read_to_string(&config_path).await?)
        .context(format!("Failed to read {config_path}"))?;
    let feeds = config.feeds;

    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        let existing_ids: HashSet<String> = table
            .iter()?
            .map(|e| Ok(e?.0.value().to_string()))
            .collect::<Result<_>>()?;

        let configured_ids: HashSet<String> = feeds.keys().cloned().collect();

        for (feed_id, mut feed) in feeds {
            if let Some(existing_guard) = table.get(feed_id.as_str())?
                && let Ok(stored) = decode_feed_data(existing_guard.value())
            {
                if feed.url.is_empty() {
                    feed.url = stored.url;
                }
                if feed.url_rss.is_none() {
                    feed.url_rss = stored.url_rss;
                }
                feed.title = stored.title;
                feed.entries = stored.entries;
                feed.description = stored.description;
                feed.last_updated = stored.last_updated;
            }
            if matches!(&feed.source, FeedSource::Youtube)
                && (feed.url.is_empty() || feed.url_rss.is_none())
            {
                if feed.url.is_empty() {
                    feed.url = format!("https://www.youtube.com/@{feed_id}");
                }
                let channel_id = resolve_youtube_channel_id(&feed.url).await?;
                feed.url_rss = Some(format!(
                    "https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}"
                ));
            }
            table.insert(feed_id.as_str(), encode_feed_data(&feed)?.as_slice())?;
        }

        for stale_id in existing_ids.difference(&configured_ids) {
            table.remove(stale_id.as_str())?;
        }
    }
    write_txn.commit()?;
    Ok(())
}

/// Refreshes all feeds concurrently.
pub async fn refresh_all_feeds() -> Result<()> {
    let read_txn = DB.begin_read()?;
    let read_table = read_txn.open_table(FEEDS_TABLE)?;
    let feeds = read_table.iter()?.flatten();

    join_all(feeds.map(|(feed_id, bytes)| async move {
        let feed_id = feed_id.value();
        let mut feed = match decode_feed_data(bytes.value()) {
            Ok(feed) => feed,
            Err(e) => {
                warn!(feed_id = %feed_id, "Failed to decode feed: {e:#}");
                return;
            }
        };
        if let Err(e) = refresh_feed(feed_id, &mut feed).await {
            warn!(feed_id = %feed_id, "Failed to refresh feed: {e:#}");
        }
    }))
    .await;

    Ok(())
}

/// Refreshes a single feed and updates the cache.
async fn refresh_feed(feed_id: &str, feed: &mut FeedData) -> Result<()> {
    info!(feed_id = %feed_id, url = %feed.url, "Refreshing feed");

    // Fetch and parse the remote feed
    let fetch_url = feed.url_rss.as_ref().unwrap_or(&feed.url);
    let content = HTTP_CLIENT.get(fetch_url).send().await?.bytes().await?;
    let fetched = parser::parse(content.as_ref())?;

    if let Some(title) = fetched.title {
        feed.title = title.content;
    }
    if let Some(url) = fetched.links.first() {
        feed.url.clone_from(&url.href);
    }
    if let Some(description) = fetched.description {
        feed.description = description.content;
    }
    if let Some(last_updated) = fetched.updated {
        feed.last_updated = last_updated.to_rfc2822();
    }

    // Process all entries concurrently
    let entries = join_all(fetched.entries.into_iter().map(|e| build_item(feed, e))).await;
    for entry in entries.into_iter().flatten() {
        feed.entries.insert(entry.id.clone(), entry);
    }

    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        table.insert(feed_id, encode_feed_data(&*feed)?.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Builds a single RSS item, checking for existing items and optionally summarizing content.
async fn build_item(feed: &FeedData, entry: Entry) -> Result<FeedEntry> {
    // If the item already exists in our database, return None to avoid reprocessing.
    if feed.entries.contains_key(&entry.id) {
        bail!("Item already exists");
    }

    let link = entry
        .links
        .first()
        .ok_or_else(|| anyhow!("Entry has no link."))?
        .href
        .clone();
    info!(link = %link, "Loading webpage");

    let mut title = entry.title.map_or_else(|| feed.url.clone(), |t| t.content);
    let mut description = entry.content.and_then(|c| c.body).unwrap_or_default();

    match &feed.source {
        FeedSource::Website => {
            // Try to fetch the full article content.
            if let Ok(response) = HTTP_CLIENT.get(&link).send().await
                && let Ok(description_html) = response.text().await
            {
                write!(
                    description,
                    "\n\n\nOriginal article full html page:\n{description_html}",
                )?;
            }

            summarise_website(&link, feed, &mut title, &mut description).await?;
        }
        FeedSource::Youtube => {
            if let Some(media_description) = entry.media.iter().find_map(|m| m.description.as_ref())
            {
                description.clone_from(&media_description.content);
            }
            summarise_youtube(&link, feed, &mut title, &mut description).await?;
        }
        _ => {}
    }

    Ok(FeedEntry {
        id: entry.id.clone(),
        title,
        link,
        description,
        published: entry
            .published
            .map_or_else(|| Utc::now().to_rfc3339(), |date| date.to_rfc2822()),
        author: entry
            .authors
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<String>>()
            .join(", "),
    })
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SummariseOutput {
    /// The rewritten title, kept concise and descriptive.
    pub title: String,
    /// Summarised content of the article, well written and engaging. Outputted as correct HTML, including images (in a figure with caption where possible) and links.
    pub content: String,
}

/// Summarises a website article.
async fn summarise_website(
    link: &str,
    feed: &FeedData,
    title: &mut String,
    description: &mut String,
) -> Result<()> {
    if !feed.clean_title && !feed.summarise_content {
        return Ok(());
    }

    let body = HTTP_CLIENT.get(link).send().await?.text().await?;
    let cleaned = html2text::from_read(body.as_bytes(), 120).unwrap_or_default();
    let page_content = if cleaned.trim().is_empty() {
        description.clone()
    } else {
        cleaned
    };

    // Summarize the content
    match run::<SummariseOutput>(
        vec![
            format!("Original title: {title}"),
            format!("Original content: {page_content}"),
        ],
        "Rewrite this rss feed entry. Content well formatted in paragraphs.".to_string(),
    )
    .await
    {
        Ok(output) => {
            if feed.clean_title {
                *title = output.title;
            }
            if feed.summarise_content {
                *description = output.content;
            }
            Ok(())
        }
        Err(err) => {
            warn!(
                link = %link,
                "Summarization failed: {err:#}"
            );
            bail!(err);
        }
    }
}

/// Summarises a youtube video from its captions.
async fn summarise_youtube(
    link: &str,
    feed: &FeedData,
    title: &mut String,
    description: &mut String,
) -> Result<()> {
    if !feed.clean_title && !feed.summarise_content {
        return Ok(());
    }

    // Get the video ID from the link
    let Some(video_id_part) = link.split("v=").nth(1) else {
        // Invalid youtube link, probably a youtube short
        return Ok(());
    };
    let video_id = video_id_part.split('&').next().unwrap_or(video_id_part);

    // Load youtube captions
    let caption_link = format!("{INVIDIOUS_API_URL}/captions/{video_id}?label=English");
    info!(link = %caption_link, "Loading youtube captions");
    let response = HTTP_CLIENT.get(caption_link).send().await?;
    let captions = response.text().await.unwrap_or_default();

    let cleaned_captions = captions
        .lines()
        .filter(|line| !line.contains("-->"))
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .replace("  ", " ")
        .to_string();

    // Summarize the content
    let system_context = vec![
        format!("Original title: {title}"),
        format!("Original description: {description}"),
        format!("Videos captions: {cleaned_captions}"),
    ];
    match run::<SummariseOutput>(system_context, "Rewrite this youtube video title and description. The title to be a more accurate description removing clickbait questions etc while preserving the original tone, meaning and fun. The description should accurately summarize the video based on its captions, it should contain a few sentences with a few concise summary, then (only if needed) two new lines then an expansion of the summary which is still kept simple.".to_string()).await {
        Ok(output) => {
            if feed.clean_title {
                *title = output.title;
            }
            if feed.summarise_content {
                *description = output.content;
            }
            Ok(())
        }
        Err(err) => {
            warn!(
                link = %link,
                "Summarization failed: {err:#}"
            );
            bail!(err)
        }
    }
}

/// Helper function to retrieve and decode `FeedData` from the database.
fn get_feed_data_from_db(id: &str) -> Result<FeedData> {
    let read_txn = DB
        .begin_read()
        .context("Failed to begin read transaction")?;
    let read_table = read_txn
        .open_table(FEEDS_TABLE)
        .context("Failed to open feeds table")?;
    let channel_bytes_guard = read_table
        .get(id)?
        .ok_or_else(|| anyhow!("Feed with ID '{id}' not found"))?;
    let feed_data =
        decode_feed_data(channel_bytes_guard.value()).context("Failed to decode feed data")?;
    Ok(feed_data)
}

/// Axum handler to serve the generated feed.
pub async fn summarised_feed_handler(Path(id): Path<String>) -> Response {
    let feed_data = match get_feed_data_from_db(&id) {
        Ok(data) => data,
        Err(e) => {
            warn!(feed_id = %id, "Failed to retrieve feed data: {e:#}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let rss_channel = feed_data.to_rss_channel();
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/rss+xml; charset=utf-8")
        .body(rss_channel.to_string().into())
        .unwrap_or_else(|_| {
            warn!(feed_id = %id, "Failed to build RSS response body");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}
