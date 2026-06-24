use crate::HTTP_CLIENT;
use anyhow::Result;
use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::LazyLock,
};
use tracing::{info, warn};

static MINIFLUX_URL: LazyLock<String> = LazyLock::new(|| env::var("MINIFLUX_URL").unwrap());
static MINIFLUX_KEY: LazyLock<String> = LazyLock::new(|| env::var("MINIFLUX_KEY").unwrap());

#[derive(Debug, Clone)]
pub struct FeedTarget {
    pub name: String,
    pub category: String,
    pub feed_url: String,
    pub site_url: Option<String>,
}

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

async fn get_api<T>(path: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = HTTP_CLIENT
        .request(Method::GET, format!("{}/{path}", *MINIFLUX_URL))
        .header("X-Auth-Token", MINIFLUX_KEY.as_str())
        .header("Content-Type", "application/json")
        .send()
        .await?
        .bytes()
        .await?;

    Ok(serde_json::from_slice::<T>(&bytes)?)
}

async fn send_api(method: Method, path: &str, body: Option<Value>) -> Result<()> {
    let mut request = HTTP_CLIENT
        .request(method.clone(), format!("{}/{path}", *MINIFLUX_URL))
        .header("X-Auth-Token", MINIFLUX_KEY.as_str())
        .header("Content-Type", "application/json");

    if let Some(body) = body {
        request = request.body(serde_json::to_vec(&body)?);
    }

    if !cfg!(debug_assertions)
        && let Err(err) = request.send().await?.error_for_status()
    {
        warn!("Miniflux {} {path} failed: {err}", method.as_str());
    }

    Ok(())
}

pub async fn update_feeds<'a>(
    wanted_categories: impl Iterator<Item = &'a String>,
    feed_targets: Vec<FeedTarget>,
) -> Result<()> {
    let wanted_categories = wanted_categories.cloned().collect::<HashSet<_>>();
    let targets_by_url = feed_targets
        .into_iter()
        .map(|target| (target.feed_url.clone(), target))
        .collect::<HashMap<_, _>>();

    let categories = sync_categories(&wanted_categories).await?;
    let feeds = get_api::<Vec<MinifluxFeed>>("v1/feeds").await?;

    add_missing_feeds(&targets_by_url, &categories, &feeds).await?;

    let feeds = get_api::<Vec<MinifluxFeed>>("v1/feeds").await?;
    update_or_delete_feeds(&targets_by_url, &categories, &wanted_categories, &feeds).await?;
    delete_stale_categories(&categories, &wanted_categories).await
}

async fn sync_categories(wanted_categories: &HashSet<String>) -> Result<Vec<MinifluxCategory>> {
    let mut categories = get_api::<Vec<MinifluxCategory>>("v1/categories").await?;
    let existing_names = categories
        .iter()
        .map(|category| category.title.clone())
        .collect::<HashSet<_>>();

    for category in wanted_categories.difference(&existing_names) {
        info!("Adding category: {category}");
        send_api(
            Method::POST,
            "v1/categories",
            Some(json!({ "title": category })),
        )
        .await?;
    }

    if !wanted_categories.is_subset(&existing_names) {
        categories = get_api::<Vec<MinifluxCategory>>("v1/categories").await?;
    }

    Ok(categories)
}

async fn add_missing_feeds(
    targets_by_url: &HashMap<String, FeedTarget>,
    categories: &[MinifluxCategory],
    feeds: &[MinifluxFeed],
) -> Result<()> {
    let category_ids = category_ids(categories);

    for target in targets_by_url.values() {
        let Some(category_id) = category_ids.get(&target.category) else {
            continue;
        };
        if feeds.iter().any(|feed| feed.feed_url == target.feed_url) {
            continue;
        }

        info!("Adding feed: {} {}", target.name, target.feed_url);
        send_api(
            Method::POST,
            "v1/feeds",
            Some(json!({
                "feed_url": target.feed_url,
                "category_id": category_id,
            })),
        )
        .await?;
    }

    Ok(())
}

async fn update_or_delete_feeds(
    targets_by_url: &HashMap<String, FeedTarget>,
    categories: &[MinifluxCategory],
    wanted_categories: &HashSet<String>,
    feeds: &[MinifluxFeed],
) -> Result<()> {
    let category_ids = category_ids(categories);

    for feed in feeds {
        if let Some(target) = targets_by_url.get(&feed.feed_url) {
            update_feed_if_needed(feed, target, &category_ids).await?;
        } else if wanted_categories.contains(&feed.category.title) {
            info!("Deleting unmanaged feed: {}", feed.title);
            send_api(Method::DELETE, &format!("v1/feeds/{}", feed.id), None).await?;
        } else {
            info!(
                "Deleting feed from stale category {}: {}",
                feed.category.title, feed.title
            );
            send_api(Method::DELETE, &format!("v1/feeds/{}", feed.id), None).await?;
        }
    }

    Ok(())
}

async fn update_feed_if_needed(
    feed: &MinifluxFeed,
    target: &FeedTarget,
    category_ids: &HashMap<String, usize>,
) -> Result<()> {
    let Some(category_id) = category_ids.get(&target.category) else {
        return Ok(());
    };
    let site_url_changed = target
        .site_url
        .as_ref()
        .is_some_and(|site_url| feed.site_url != *site_url);

    if feed.category.id == *category_id && !site_url_changed {
        return Ok(());
    }

    info!("Updating feed: {}", feed.title);
    let mut body = json!({ "category_id": category_id });
    if let Some(site_url) = &target.site_url {
        body["site_url"] = json!(site_url);
    }
    send_api(Method::PUT, &format!("v1/feeds/{}", feed.id), Some(body)).await
}

async fn delete_stale_categories(
    categories: &[MinifluxCategory],
    wanted_categories: &HashSet<String>,
) -> Result<()> {
    for category in categories {
        if wanted_categories.contains(&category.title) {
            continue;
        }

        info!("Deleting stale category: {}", category.title);
        send_api(
            Method::DELETE,
            &format!("v1/categories/{}", category.id),
            None,
        )
        .await?;
    }

    Ok(())
}

fn category_ids(categories: &[MinifluxCategory]) -> HashMap<String, usize> {
    categories
        .iter()
        .map(|category| (category.title.clone(), category.id))
        .collect()
}
