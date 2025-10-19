use crate::feed::{FeedConfigFile, HTTP_CLIENT};
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

#[derive(Debug, Deserialize)]
struct MinifluxCategory {
    id: usize,
    title: String,
    user_id: usize,
    hide_globally: bool,
}

#[derive(Debug, Deserialize)]
struct MinifluxFeed {
    id: usize,
    user_id: usize,
    title: String,
    site_url: String,
    feed_url: String,
    checked_at: String,
    etag_header: String,
    last_modified_header: String,
    parsing_error_message: String,
    parsing_error_count: usize,
    scraper_rules: String,
    rewrite_rules: String,
    crawler: bool,
    blocklist_rules: String,
    keeplist_rules: String,
    user_agent: String,
    username: String,
    password: String,
    disabled: bool,
    ignore_http_cache: bool,
    fetch_via_proxy: bool,
    category: MinifluxCategory,
    icon: MinifluxIcon,
}

#[derive(Debug, Deserialize)]
struct MinifluxIcon {
    feed_id: usize,
    icon_id: usize,
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

async fn delete_api(url: &str) -> Result<String, reqwest::Error> {
    api(Method::DELETE, url, None).await?.text().await
}

pub async fn update_feeds(config_file: &FeedConfigFile) -> Result<()> {
    let wanted_categories: HashSet<String> = config_file.keys().cloned().collect();

    // Get existing categories on miniflux
    let categories = query_api::<Vec<MinifluxCategory>>("v1/categories").await?;
    let categories_names: HashSet<String> = categories.iter().map(|c| c.title.clone()).collect();
    let category_ids: HashMap<String, usize> =
        categories.iter().map(|c| (c.title.clone(), c.id)).collect();
    println!("Categories: {categories:?}");

    // Add missing categories
    for category in wanted_categories.difference(&categories_names) {
        info!("Adding category: {category}");
        let response = post_api("v1/categories", json!({ "title": category })).await;
        println!("Response: {response:?}");
    }

    // Remove excess categories
    for category in categories_names.difference(&wanted_categories) {
        let id = category_ids.get(category).unwrap();
        info!("Removing category: {category} {id}");
        // delete_api(&format!("v1/categories/{id}")).await?;
    }

    // let response = query_api::<Vec<MinifluxFeed>>("v1/feeds").await?;
    // println!("Feeds: {response:?}");

    Ok(())
}
