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
use json_feed_model::{Author, Feed, Item, Version};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use reqwest::{Client, header::CONTENT_TYPE};
use schemars::JsonSchema;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Display,
    sync::LazyLock,
};
use tokio::fs;
use tracing::{info, warn};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
const INVIDIOUS_API_URL: &str = "https://inv.nadeko.net/api/v1";

const SUMMARISE_WEBSITE: &str = "Rewrite this rss feed entry outputted as embedded HTML. Content well formatted in paragraphs, written in same the article style as the original just trimmed and concise. Include at least one image (in a figure with caption where possible, no alt text) using the original image url. The first image is used as the thumbnail and should be placed after at least the first paragraph of text. Include extra images if available at the end. Also include inline links where appropriate.";
const SUMMARISE_YOUTUBE: &str = "Rewrite this youtube video title and description. The title to be a more accurate description removing clickbait questions etc while preserving the original tone, meaning and fun. The description should accurately summarise the video based on its captions, it should contain a few sentences with a few concise summary, then (only if needed) two new lines then an expansion of the summary which is still kept simple.";

const FEEDS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("feeds");
static DB: LazyLock<Database> = LazyLock::new(|| {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.redb".to_string());
    Database::create(database_url).unwrap()
});

#[derive(Deserialize)]
struct FeedConfig {
    #[serde(default)]
    source: FeedSource,
    filters: Option<Vec<String>>,
    url_rss: Option<String>,

    #[serde(default)]
    clean_title: bool,
    #[serde(default)]
    summarise_content: bool,
}

#[derive(Encode, Decode, Default)]
struct FeedData {
    source: FeedSource,
    title: String,
    url: String,
    url_rss: String,

    filters: Vec<String>,
    clean_title: bool,
    summarise_content: bool,

    description: String,
    last_updated: String,
    favicon: Option<String>,
    icon: Option<String>,
    authors: Vec<String>,
    tags: Vec<String>,
    entries: HashMap<String, FeedEntry>,
}

impl FeedData {
    fn to_json_feed(&self) -> Feed {
        // Sort by date_published, recent first
        let mut items = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.title != "INVALID")
            .collect::<Vec<_>>();
        items.sort_by(|(_, a), (_, b)| {
            let key_a = DateTime::parse_from_rfc3339(&a.published).ok();
            let key_b = DateTime::parse_from_rfc3339(&b.published).ok();
            key_b.cmp(&key_a)
        });
        let items = items
            .iter()
            .map(|(i, e)| e.to_json_item(i))
            .collect::<Vec<_>>();

        let mut feed = Feed::new();
        feed.set_version(Version::Version1_1);
        feed.set_title(&self.title);
        feed.set_home_page_url(&self.url);
        feed.set_feed_url(&self.url_rss);
        feed.set_description(&self.description);
        feed.set_items(items);
        if let Some(icon) = &self.icon {
            feed.set_icon(icon);
        }
        if let Some(favicon) = &self.favicon {
            feed.set_favicon(favicon);
        }

        assert!(feed.is_valid(&Version::Version1_1));

        feed
    }
}

#[derive(Encode, Decode, Default)]
struct FeedEntry {
    title: String,
    link: String,
    description: String,
    published: String,
    image: String,
    authors: Vec<String>,
    tags: Vec<String>,
}

impl FeedEntry {
    fn to_json_item(&self, id: &str) -> Item {
        let mut item = Item::new();
        item.set_id(id);
        item.set_title(&self.title);
        item.set_url(&self.link);
        item.set_content_html(&self.description);
        item.set_date_published(&self.published);
        item.set_authors(self.authors.iter().map(|a| {
            let mut author = Author::new();
            author.set_name(a);
            author
        }));
        item.set_banner_image(&self.image);
        item.set_tags(self.tags.clone());
        item
    }

    fn invalid() -> Self {
        Self {
            title: "INVALID".to_string(),
            ..Default::default()
        }
    }
}

#[derive(Encode, Decode, Deserialize, Copy, Clone, Default)]
enum FeedSource {
    #[default]
    Website,
    Youtube,
    Twitter,
    Reddit,
}

impl Display for FeedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Website => write!(f, "Website"),
            Self::Youtube => write!(f, "Youtube"),
            Self::Twitter => write!(f, "Twitter"),
            Self::Reddit => write!(f, "Reddit"),
        }
    }
}

/// Helper function to encode `FeedData`
fn encode_feed_data(feed_data: &FeedData) -> Result<Vec<u8>> {
    Ok(bincode::encode_to_vec(feed_data, config::standard())?)
}

/// Helper function to decode `FeedData`
fn decode_feed_data(bytes: &[u8]) -> Result<FeedData> {
    Ok(bincode::decode_from_slice(bytes, config::standard())?.0)
}

/// Ensure the `redb` database is ready for use.
pub async fn init_storage() -> Result<()> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.json".to_string());
    let config: HashMap<String, FeedConfig> =
        serde_json::from_str(&fs::read_to_string(&config_path).await?)
            .context(format!("Failed to read {config_path}"))?;
    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        let existing_ids: HashSet<String> = table
            .iter()?
            .map(|e| Ok(e?.0.value().to_string()))
            .collect::<Result<_>>()?;

        let configured_ids: HashSet<String> = config.keys().cloned().collect();

        for (feed_id, config_feed) in config {
            // Get stored feed data
            let mut feed = if let Some(existing_guard) = table.get(feed_id.as_str())?
                && let Ok(stored) = decode_feed_data(existing_guard.value())
            {
                stored
            } else {
                FeedData::default()
            };

            // Override with the config
            feed.source = config_feed.source;
            feed.clean_title = config_feed.clean_title;
            feed.summarise_content = config_feed.summarise_content;
            if let Some(filters) = config_feed.filters {
                feed.filters = filters;
            }
            if let Some(url_rss) = config_feed.url_rss {
                feed.url_rss = url_rss;
            }

            // Update the youtube urls
            if matches!(&config_feed.source, FeedSource::Youtube) {
                if feed.url.is_empty() {
                    feed.url = format!("https://www.youtube.com/@{feed_id}");
                }
                if feed.url_rss.is_empty() {
                    let channel_id = resolve_youtube_channel_id(&feed.url).await?;
                    feed.url_rss =
                        format!("https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}");
                }
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
    let content = HTTP_CLIENT.get(&feed.url_rss).send().await?.bytes().await?;
    let fetched = parser::parse(content.as_ref())?;

    if let Some(title) = fetched.title {
        feed.title = title.content;
    }
    if let Some(url) = fetched
        .links
        .iter()
        .find(|link| link.media_type.as_deref() == Some("text/html"))
        .or_else(|| fetched.links.first())
    {
        feed.url.clone_from(&url.href);
    }
    if let Some(description) = fetched.description {
        feed.description = description.content;
    }
    if let Some(last_updated) = fetched.updated {
        feed.last_updated = last_updated.to_rfc3339();
    }
    feed.icon = fetched.logo.map(|icon| icon.uri);
    feed.favicon = fetched.icon.map(|icon| icon.uri);
    feed.authors = fetched.authors.iter().map(|a| a.name.clone()).collect();
    feed.tags = fetched.categories.iter().map(|c| c.term.clone()).collect();

    // Process all entries concurrently
    let entries = join_all(fetched.entries.into_iter().map(|e| build_item(feed, e))).await;
    for (entry_id, entry) in entries.into_iter().flatten() {
        feed.entries.insert(entry_id, entry);
    }

    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        table.insert(feed_id, encode_feed_data(&*feed)?.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Builds a single RSS item, checking for existing items and optionally summarising content.
async fn build_item(feed: &FeedData, entry: Entry) -> Result<(String, FeedEntry)> {
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
    info!(link = %link, "Loading {}", feed.source);

    let mut title = entry.title.map_or_else(|| feed.url.clone(), |t| t.content);
    let mut description = entry.content.and_then(|c| c.body).unwrap_or_default();

    let summarised = match &feed.source {
        FeedSource::Youtube => {
            if let Some(media_description) = entry.media.iter().find_map(|m| m.description.as_ref())
            {
                description.clone_from(&media_description.content);
            }

            // Get the video ID from the link
            let Some(video_id_part) = link.split("v=").nth(1) else {
                // Invalid youtube link, probably a youtube short
                info!("FILTERED: Youtube video {link}");
                return Ok((entry.id, FeedEntry::invalid()));
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

            summarise_content(
                &link,
                feed,
                &title,
                &description,
                SUMMARISE_YOUTUBE,
                "Videos captions:",
                &cleaned_captions,
            )
            .await?
        }
        _ => {
            let page_content = HTTP_CLIENT.get(&link).send().await?.text().await?;
            let summarised = summarise_content(
                &link,
                feed,
                &title,
                &description,
                SUMMARISE_WEBSITE,
                "Original content:",
                &page_content,
            )
            .await?;

            // Save the description as a html file in outputs
            #[cfg(debug_assertions)]
            fs::write(
                format!("outputs/{}.html", title.to_lowercase().replace(' ', "_")),
                description.clone(),
            )
            .await?;

            summarised
        }
    };

    if feed.clean_title {
        title = summarised.title;
    }
    if feed.summarise_content {
        description = summarised.content;
    }

    if !feed.filters.is_empty() && !summarised.included {
        info!("FILTERED: {link}");
        return Ok((entry.id, FeedEntry::invalid()));
    }

    Ok((
        entry.id,
        FeedEntry {
            title,
            link,
            description,
            published: entry.published.unwrap_or_else(Utc::now).to_rfc3339(),
            authors: entry.authors.iter().map(|p| p.name.clone()).collect(),
            tags: entry.categories.iter().map(|c| c.term.clone()).collect(),
            image: summarised.image,
        },
    ))
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct SummariseOutput {
    /// The rewritten title, kept concise and descriptive.
    title: String,
    /// Summarised content, well written and engaging.
    content: String,
    /// Key image url, to represent the feed as a thumbnail
    image: String,
    /// Whether this entry should be included, false for filtered out.
    included: bool,
}

/// Summarises content for a feed entry.
async fn summarise_content(
    link: &str,
    feed: &FeedData,
    title: &str,
    description: &str,
    prompt: &str,
    content_key: &str,
    original_content: &str,
) -> Result<SummariseOutput> {
    match run::<SummariseOutput>(
        {
            let mut context = vec![
                format!("Original title: {title}"),
                format!("Original description: {description}"),
                format!("{content_key} {original_content}"),
            ];
            if !feed.filters.is_empty() {
                context.push(format!(
                    "Users requested filters, do not include posts including any of these '{}'",
                    feed.filters.join(";")
                ));
            }
            context
        },
        prompt,
    )
    .await
    {
        Ok(output) => Ok(output),
        Err(err) => {
            warn!(
                link = %link,
                "Summarisation failed: {err:#}"
            );
            bail!(err);
        }
    }
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
    let feed_string = match serde_json::to_string(&feed_data.to_json_feed()) {
        Ok(data) => data,
        Err(e) => {
            warn!(feed_id = %id, "Failed to serialize feed data: {e:#}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/rss+xml; charset=utf-8")
        .body(feed_string.into())
        .unwrap_or_else(|_| {
            warn!(feed_id = %id, "Failed to build RSS response body");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}
