use crate::llm_functions;
use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bincode::{Decode, Encode, config};
use chrono::Utc;
use feed_rs::{model::Entry, parser};
use futures::future::join_all;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use reqwest::{Client, header::CONTENT_TYPE};
use rss::{Guid, ItemBuilder};
use std::{collections::HashMap, env, sync::LazyLock};
use tracing::{info, warn};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

pub const FEEDS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("feeds");
pub static DB: LazyLock<Database> = LazyLock::new(|| {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.redb".to_string());
    Database::create(database_url).unwrap()
});

#[derive(Encode, Decode)]
pub struct FeedData {
    pub id: String,
    pub title: String,
    pub url: String,
    pub url_rss: Option<String>,
    pub description: Option<String>,
    pub last_updated: Option<String>,
    pub config: FeedConfig,
    pub entries: HashMap<String, FeedEntry>,
}

impl FeedData {
    pub fn to_rss_channel(&self) -> rss::Channel {
        let mut channel_builder = rss::ChannelBuilder::default();
        channel_builder
            .title(self.title.clone())
            .link(self.url.clone())
            .description(self.description.clone().unwrap_or_default())
            .items(
                self.entries
                    .values()
                    .map(FeedEntry::to_rss_item)
                    .collect::<Vec<rss::Item>>(),
            );

        if let Some(last_updated) = &self.last_updated {
            channel_builder.last_build_date(Some(last_updated.clone()));
        }

        channel_builder.build()
    }
}

#[derive(Encode, Decode, Clone)]
pub struct FeedEntry {
    pub id: String,
    pub title: String,
    pub link: String,
    pub description: String,
    pub published: String,
}

impl FeedEntry {
    pub fn to_rss_item(&self) -> rss::Item {
        let mut builder = ItemBuilder::default();
        builder
            .title(Some(self.title.clone()))
            .link(Some(self.link.clone()))
            .description(Some(self.description.clone()))
            .guid(Guid {
                value: self.id.clone(),
                permalink: true,
            });
        builder.pub_date(Some(self.published.clone()));
        builder.build()
    }
}

#[derive(Encode, Decode)]
pub enum FeedConfig {
    Website(WebsiteFeed),
    Youtube(YoutubeFeed),
    Twitter(TwitterFeed),
    Reddit(RedditFeed),
}

#[derive(Encode, Decode)]
pub struct WebsiteFeed {
    pub clean_title: bool,
    pub summarise_articles: bool,
}

#[derive(Encode, Decode)]
pub struct YoutubeFeed {
    pub clean_title: bool,
    pub summarise_videos: bool,
}

#[derive(Encode, Decode)]
pub struct TwitterFeed {
    pub summarise_threads: bool,
}

#[derive(Encode, Decode)]
pub struct RedditFeed {}

// Helper function to encode FeedData
fn encode_feed_data(feed_data: &FeedData) -> Result<Vec<u8>> {
    Ok(bincode::encode_to_vec(feed_data, config::standard())?)
}

// Helper function to decode FeedData
fn decode_feed_data(bytes: &[u8]) -> Result<FeedData> {
    Ok(bincode::decode_from_slice(bytes, config::standard())?.0)
}

/// Ensure the `redb` database is ready for use.
pub async fn init_storage() -> Result<()> {
    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        table.insert(
            "verge",
            encode_feed_data(&FeedData {
                id: "verge".to_string(),
                title: "The Verge".to_string(),
                url: "https://www.theverge.com".to_string(),
                url_rss: Some("https://www.theverge.com/rss/index.xml".to_string()),
                description: None,
                last_updated: None,
                config: FeedConfig::Website(WebsiteFeed {
                    clean_title: true,
                    summarise_articles: true,
                }),
                entries: HashMap::new(),
            })?
            .as_slice(),
        )?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Refreshes all feeds concurrently.
pub async fn refresh_all_feeds() -> Result<()> {
    let read_txn = DB.begin_read()?;
    let read_table = read_txn.open_table(FEEDS_TABLE)?;
    let table_iter = read_table.iter()?;

    let refresh_tasks = table_iter.map(|entry_result| async move {
        let (_id_ref, channel_bytes_ref) = entry_result?;
        let mut feed: FeedData = decode_feed_data(channel_bytes_ref.value())?;
        if let Err(err) = refresh_feed(&mut feed).await {
            warn!(feed_id = %feed.id, "Failed to refresh feed: {err:#}");
        }
        Ok::<(), anyhow::Error>(())
    });

    // Await all refresh tasks to complete.
    join_all(refresh_tasks).await;

    Ok(())
}

/// Refreshes a single feed and updates the cache.
async fn refresh_feed(feed: &mut FeedData) -> Result<()> {
    info!(feed_id = %feed.id, url = %feed.url, "Refreshing feed");

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
        feed.description = Some(description.content);
    }
    if let Some(last_updated) = fetched.updated {
        feed.last_updated = Some(last_updated.to_rfc2822());
    }

    // Process all entries concurrently
    let entries = join_all(
        fetched
            .entries
            .into_iter()
            .map(|entry| build_item(feed, entry)),
    );
    for entry in entries.await.into_iter().flatten() {
        feed.entries.insert(entry.id.clone(), entry);
    }

    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        table.insert(feed.id.as_str(), encode_feed_data(&*feed)?.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Builds a single RSS item, checking for existing items and optionally summarizing content.
async fn build_item(feed: &FeedData, entry: Entry) -> Option<FeedEntry> {
    let link = entry.links.first()?.href.clone();
    info!(link = %link, "Loading webpage");

    // If the item already exists in our database, return None to avoid reprocessing.
    if feed.entries.contains_key(&entry.id) {
        return None;
    }

    // Try to fetch the full article content; fall back to the feeds summary.
    let description_html = async { HTTP_CLIENT.get(&link).send().await?.text().await }
        .await
        .ok()
        .unwrap_or_else(|| entry.content.and_then(|c| c.body).unwrap_or_default());

    let mut title = entry
        .title
        .as_ref()
        .map_or_else(|| feed.url.to_string(), |t| t.content.to_string());
    let mut description =
        html2text::from_read(description_html.as_bytes(), 120).unwrap_or_default();

    if let FeedConfig::Website(website_feed) = &feed.config
        && (website_feed.clean_title || website_feed.summarise_articles)
    {
        match llm_functions::summarise_article(&title, &description).await {
            Ok(output) => {
                if website_feed.clean_title {
                    title = output.title;
                }
                if website_feed.summarise_articles {
                    description = output.content;
                }
            }
            Err(err) => {
                warn!(
                    feed_id = %feed.id,
                    entry_id = %entry.id,
                    "Summarization failed: {err:#}"
                );
            }
        }
    }

    Some(FeedEntry {
        id: entry.id.clone(),
        title,
        link,
        description,
        published: entry
            .published
            .map_or_else(|| Utc::now().to_rfc3339(), |date| date.to_rfc2822()),
    })
}

/// Helper function to retrieve and decode `FeedData` from the database.
fn get_feed_data_from_db(id: &str) -> Result<FeedData, StatusCode> {
    let read_txn = DB
        .begin_read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let read_table = read_txn
        .open_table(FEEDS_TABLE)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let channel_bytes_guard = read_table
        .get(id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let feed_data = decode_feed_data(channel_bytes_guard.value())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(feed_data)
}

/// Axum handler to serve the generated feed.
pub async fn summarised_feed_handler(Path(id): Path<String>) -> Response {
    let feed_data = match get_feed_data_from_db(&id) {
        Ok(data) => data,
        Err(status_code) => return status_code.into_response(),
    };

    let rss_channel = feed_data.to_rss_channel();
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/rss+xml; charset=utf-8")
        .body(rss_channel.to_string().into())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
