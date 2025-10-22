use crate::feed::{FeedConfigFile, FeedData, HTTP_CLIENT};
use anyhow::{Ok, Result};
use reqwest::Method;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::LazyLock,
};
use tracing::info;

static MINIFLUX_URL: LazyLock<String> = LazyLock::new(|| env::var("MINIFLUX_URL").unwrap());
static MINIFLUX_KEY: LazyLock<String> = LazyLock::new(|| env::var("MINIFLUX_KEY").unwrap());
static QUIVRS_URL: LazyLock<String> = LazyLock::new(|| env::var("QUIVRS_URL").unwrap());
static QUIVRS_URL_PREFIX: LazyLock<String> = LazyLock::new(|| format!("{}/feeds/", *QUIVRS_URL));

#[derive(Debug, Deserialize)]
struct MinifluxCategory {
    id: usize,
    title: String,
}

#[derive(Debug, Deserialize)]
struct MinifluxFeed {
    id: usize,
    title: String,
    site_url: String,
    feed_url: String,
    category: MinifluxCategory,
}

async fn api(
    method: Method,
    url: &str,
    body: Option<&Value>,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut request = HTTP_CLIENT
        .request(method, format!("{}/{url}", *MINIFLUX_URL))
        .header("X-Auth-Token", MINIFLUX_KEY.as_str())
        .header("Content-Type", "application/json");
    if let Some(body) = body {
        request = request.json(&body);
    }
    request.send().await
}

async fn query_api<T>(url: &str) -> Result<T, reqwest::Error>
where
    T: DeserializeOwned,
{
    api(Method::GET, url, None).await?.json::<T>().await
}

async fn post_api(url: &str, body: Value) -> Result<String, reqwest::Error> {
    api(Method::POST, url, Some(&body)).await?.text().await
}

async fn put_api(url: &str, body: Value) -> Result<String, reqwest::Error> {
    api(Method::PUT, url, Some(&body)).await?.text().await
}

async fn delete_api(url: &str) -> Result<String, reqwest::Error> {
    api(Method::DELETE, url, None).await?.text().await
}

pub async fn update_feeds(
    config_feeds: &FeedConfigFile,
    database_feeds: HashMap<String, FeedData>,
) -> Result<()> {
    let wanted_categories: HashSet<String> = config_feeds.keys().cloned().collect();

    // Get existing categories on miniflux
    let miniflux_categories = query_api::<Vec<MinifluxCategory>>("v1/categories").await?;
    let miniflux_categories_names: HashSet<String> = miniflux_categories
        .iter()
        .map(|c| c.title.clone())
        .collect();
    let miniflux_category_ids: HashMap<String, usize> = miniflux_categories
        .iter()
        .map(|c| (c.title.clone(), c.id))
        .collect();

    // Add missing categories
    for category in wanted_categories.difference(&miniflux_categories_names) {
        info!("Adding category: {category}");
        post_api("v1/categories", json!({ "title": category })).await?;
    }

    // Remove excess categories
    for category in miniflux_categories_names.difference(&wanted_categories) {
        if let Some(id) = miniflux_category_ids.get(category) {
            info!("Removing category: {category} {id}");
            delete_api(&format!("v1/categories/{id}")).await?;
        }
    }

    // Get existing feeds
    let miniflux_feeds = query_api::<Vec<MinifluxFeed>>("v1/feeds").await?;

    // Add missing feeds
    for (feed_id, database_feed) in &database_feeds {
        let feed_url = format!("{}{}", *QUIVRS_URL_PREFIX, feed_id);
        let Some(Some(category_id)) = config_feeds
            .iter()
            .find(|(_, inner_map)| inner_map.contains_key(feed_id))
            .map(|(category, _)| miniflux_category_ids.get(category))
        else {
            continue;
        };
        if !miniflux_feeds.iter().any(|feed| feed.feed_url == feed_url) {
            info!(
                "Adding feed: {} {} {}",
                database_feed.title, feed_url, category_id
            );
            post_api(
                "v1/feeds",
                json!({
                    "feed_url": feed_url,
                    "category_id": category_id,
                }),
            )
            .await?;
        }
    }

    // If title, site_url or category of the feed differs from our local data, update it
    let mut miniflux_used_feeds: HashSet<usize> = HashSet::new();
    for miniflux_feed in &miniflux_feeds {
        // Get associated config and config_feeds
        let Some(feed_id) = miniflux_feed.feed_url.strip_prefix(&*QUIVRS_URL_PREFIX) else {
            continue;
        };
        let Some(database_feed) = database_feeds.get(feed_id) else {
            continue;
        };
        let Some(Some(category_id)) = config_feeds
            .iter()
            .find(|(_, inner_map)| inner_map.contains_key(feed_id))
            .map(|(category, _)| miniflux_category_ids.get(category))
        else {
            continue;
        };
        miniflux_used_feeds.insert(miniflux_feed.id);

        // Update feed data if necessary
        if miniflux_feed.title != database_feed.title
            || miniflux_feed.category.id != *category_id
            || miniflux_feed.site_url != database_feed.url
        {
            info!("Updating feed: {}", miniflux_feed.title);
            put_api(
                &format!("v1/feeds/{}", miniflux_feed.id),
                json!({
                    "title": database_feed.title,
                    "category_id": category_id,
                    "site_url": database_feed.url,
                }),
            )
            .await?;
        }
    }

    // Delete unused feeds
    for miniflux_feed in &miniflux_feeds {
        if !miniflux_used_feeds.contains(&miniflux_feed.id) {
            info!("Deleting feed: {}", miniflux_feed.title);
            delete_api(&format!("v1/feeds/{}", miniflux_feed.id)).await?;
        }
    }

    Ok(())
}
