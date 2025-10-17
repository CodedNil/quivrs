use crate::llm_functions::run;
use anyhow::{Context, Result, anyhow, bail};
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Duration, Utc};
use feed_rs::{model::Entry, parser};
use futures::future::join_all;
use json_feed_model::{Author, Feed, Item, Version};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use regex::Regex;
use reqwest::{Client, header::CONTENT_TYPE};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    error::Error,
    fmt::{Display, Write},
    sync::LazyLock,
};
use tokio::fs;
use tracing::{info, warn};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
const INVIDIOUS_API_URL: &str = "https://inv.nadeko.net/api/v1";
const NITTER_API_URL: &str = "https://nitter.privacyredirect.com";

const SUMMARISE_WEBSITE: &str = "Rewrite this rss feed entry outputted as embedded HTML. Content well formatted in paragraphs, written in same the article style as the original just trimmed and concise. Include at least one image (in a figure with caption where possible, no alt text) using the original image url. The first image is used as the thumbnail and should be placed after at least the first paragraph of text. Include extra images if available at the end. Also include inline links where appropriate.";
const SUMMARISE_YOUTUBE: &str = "Rewrite this youtube video title and description as embedded HTML. The title to be a more accurate description removing clickbait questions etc while preserving the original tone, meaning and fun. The description should accurately summarise the video based on its captions, it should contain a few sentences with a few concise summary, then (only if needed) two new lines then an expansion of the summary which is still kept simple.";
const SUMMARISE_TWITTER: &str = "Rewrite this twitter post title and description as embedded HTML. The title should be a few words that accurately describe the post. The description should stay accurate to the original but clean up the formatting.";

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
    original_title: bool, // Preserve the original title
    #[serde(default)]
    original_content: bool, // Preserve the original content
}

#[derive(Serialize, Deserialize, Default)]
struct FeedData {
    source: FeedSource,
    title: String,
    url: String,
    url_rss: String,

    filters: Vec<String>,
    original_title: bool,
    original_content: bool,

    description: String,
    last_updated: DateTime<Utc>,
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
        items.sort_by(|(_, a), (_, b)| b.published.cmp(&a.published));
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

        feed
    }
}

#[derive(Serialize, Deserialize, Default)]
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
    fn to_json_item(&self, id: &str) -> Item {
        let mut item = Item::new();
        item.set_id(id);
        item.set_title(&self.title);
        item.set_url(&self.link);
        item.set_content_html(&self.description);
        item.set_date_published(self.published);
        item.set_authors(self.authors.iter().map(|a| {
            let mut author = Author::new();
            author.set_name(a);
            author
        }));
        if let Some(image) = &self.image {
            item.set_image(image);
            item.set_banner_image(image);
        }
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

#[derive(Serialize, Deserialize, Copy, Clone, Default)]
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

        for (feed_id, config) in config {
            // Get stored feed data
            let mut feed: FeedData = table
                .get(feed_id.as_str())?
                .and_then(|g| postcard::from_bytes(g.value()).ok())
                .unwrap_or_default();

            // Override with the config
            feed.source = config.source;
            feed.original_title = config.original_title;
            feed.original_content = config.original_content;
            feed.filters = config.filters.unwrap_or(feed.filters);
            feed.url_rss = config.url_rss.unwrap_or(feed.url_rss);

            // Update the per domain urls
            match config.source {
                FeedSource::Youtube => {
                    feed.url = format!("https://www.youtube.com/@{feed_id}");
                    if feed.url_rss.is_empty() {
                        feed.url_rss = format!(
                            "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                            resolve_youtube_channel_id(&feed_id).await?
                        );
                    }
                }
                FeedSource::Twitter => {
                    feed.url = format!("https://x.com/{feed_id}");
                    feed.url_rss = format!("{NITTER_API_URL}/{feed_id}/rss");
                }
                _ => {}
            }

            table.insert(feed_id.as_str(), postcard::to_allocvec(&feed)?.as_slice())?;
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
        let mut feed = match postcard::from_bytes(bytes.value()) {
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
    if !matches!(feed.source, FeedSource::Twitter)
        && let Some(url) = fetched
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
        feed.last_updated = last_updated;
    }
    feed.icon = fetched.logo.map(|icon| icon.uri);
    feed.favicon = fetched.icon.map(|icon| icon.uri);
    feed.authors = fetched.authors.iter().map(|a| a.name.clone()).collect();
    feed.tags = fetched.categories.iter().map(|c| c.term.clone()).collect();

    // Process all entries concurrently
    let entries = join_all(
        fetched
            .entries
            .clone()
            .iter()
            .map(|e| build_item(feed_id, feed, e, &fetched.entries)),
    )
    .await;
    for (entry_id, entry) in entries.into_iter().flatten() {
        feed.entries.insert(entry_id, entry);
    }

    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        table.insert(feed_id, postcard::to_allocvec(&feed)?.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Builds a single RSS item, checking for existing items and optionally summarising content.
async fn build_item(
    feed_id: &str,
    feed: &FeedData,
    entry: &Entry,
    other_entries: &[Entry],
) -> Result<(String, FeedEntry)> {
    // If the item already exists in our database, return None to avoid reprocessing.
    if feed.entries.contains_key(&entry.id) {
        bail!("Item already exists");
    }

    let mut link = entry
        .links
        .first()
        .ok_or_else(|| anyhow!("Entry has no link."))?
        .href
        .clone();
    info!(link = %link, "Parsing {}", feed.source);

    let mut title = entry
        .title
        .clone()
        .map_or("NOT PROVIDED".to_string(), |t| t.content);
    let mut description = entry
        .content
        .clone()
        .and_then(|c| c.body)
        .or_else(|| entry.summary.clone().map(|s| s.content))
        .unwrap_or_else(|| "NOT PROVIDED".to_string());
    let published = entry.published.unwrap_or_else(Utc::now);

    // If twitter, swap the link back to twitter, and reject replies
    if matches!(feed.source, FeedSource::Twitter) {
        if title.starts_with("R to @") {
            return Ok((entry.id.clone(), FeedEntry::invalid()));
        }
        if let Some(end) = link.strip_prefix(NITTER_API_URL) {
            link = format!("https://x.com{end}");
        }

        // Grab other tweets that were posted within 5 minutes and are replies to the same user, append the descriptions to build the thread
        let mut thread: Vec<(String, DateTime<Utc>)> = other_entries
            .iter()
            .filter_map(|other| {
                let other_title = other.title.as_ref()?.content.clone();
                if !other_title.starts_with(&format!("R to @{feed_id}")) {
                    return None;
                }
                let other_time = other.published?;
                if other_time <= published || (other_time - published) >= Duration::minutes(5) {
                    return None;
                }
                Some((other_title, other_time))
            })
            .collect();

        // Sort by published time (earliest first)
        thread.sort_by_key(|(_, pub_time)| *pub_time);

        // Push the entire thread into the description
        if thread.len() > 1 {
            description.push_str("\n\n");
        }
        for (i, (body, _)) in thread.iter().enumerate() {
            writeln!(
                description,
                "Reply post [{}/{}] in thread : {body}",
                i + 1,
                thread.len()
            )?;
        }
    }

    let summarised = match &feed.source {
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
                info!("FILTERED: Youtube video {link}");
                return Ok((entry.id.clone(), FeedEntry::invalid()));
            };

            // Load youtube captions
            let caption_link = format!("{INVIDIOUS_API_URL}/captions/{video_id}?label=English");
            info!(link = %caption_link, "Loading youtube captions");
            let response = HTTP_CLIENT.get(caption_link).send().await?;
            let captions = response.text().await.unwrap_or_default();

            summarise_content(
                &link,
                feed,
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
        }
        FeedSource::Twitter => {
            let mut summarised =
                summarise_content(&link, feed, &title, &description, SUMMARISE_TWITTER, "", "")
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
            let summarised = summarise_content(
                &link,
                feed,
                &title,
                &description,
                SUMMARISE_WEBSITE,
                "Original content:",
                &HTTP_CLIENT.get(&link).send().await?.text().await?,
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

    if !feed.original_title {
        title = summarised.title;
    }
    if !feed.original_content {
        description = summarised.content;
    }

    if !feed.filters.is_empty() && !summarised.included {
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
    feed: &FeedData,
    title: &str,
    description: &str,
    prompt: &str,
    content_key: &str,
    original_content: &str,
) -> Result<SummariseOutput> {
    run::<SummariseOutput>(
        {
            let mut context = vec![
                format!("Original title: {title}"),
                format!("Original description: {description}"),
            ];
            if !content_key.is_empty() {
                context.push(format!("{content_key} {original_content}"));
            }
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
    let re = Regex::new(r#"<link\s+rel=["']?canonical["']?\s+href=["']([^"']+)["']"#).unwrap();
    if let Some(caps) = re.captures(&body)
        && let Some(channel_id) = caps[1].split('/').next_back().filter(|s| !s.is_empty())
    {
        info!("Found channel ID for {channel_name}: {channel_id}",);
        return Ok(channel_id.to_string());
    }
    bail!("No channel ID found for `{channel_name}`")
}

/// A custom error type to centralize error handling and response generation.
pub enum AppError {
    NotFound(String),
    Internal(Box<dyn Error + Send + Sync>),
}

/// Converts `AppError` into an HTTP response, handling status codes and logging.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Self::Internal(e) => {
                warn!("Internal server error: {e:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}

/// Allows any standard error to be converted into an `AppError`.
impl<E> From<E> for AppError
where
    E: Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        Self::Internal(Box::new(err))
    }
}

/// Axum handler to serve the generated feed.
pub async fn summarised_feed_handler(
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let read_txn = DB.begin_read()?;
    let table = read_txn.open_table(FEEDS_TABLE)?;
    let feed_bytes_guard = table
        .get(id.as_str())?
        .ok_or_else(|| AppError::NotFound(format!("Feed with ID '{id}' not found")))?;

    let feed_data: FeedData = postcard::from_bytes(feed_bytes_guard.value())?;
    let feed_string = serde_json::to_string(&feed_data.to_json_feed())?;

    Ok((
        StatusCode::OK,
        [(CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        feed_string,
    ))
}
