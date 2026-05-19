pub mod articles;
pub mod database;
mod embeddings;
mod llm_functions;
mod parse_feed;
mod parse_website;

use reqwest::{Client, header};
use std::{sync::LazyLock, time::Duration};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{error, info};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json, text/html, */*;q=0.8"),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        header::HeaderValue::from_static("en-GB,en;q=0.9"),
    );
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .default_headers(headers)
        .build()
        .expect("Failed to build HTTP client")
});

pub fn start() {
    tokio::spawn(async move {
        if let Err(err) = database::init().await {
            error!("Database initialisation failed: {err}");
            return;
        }
        if let Err(err) = database::regenerate_stale_embeddings().await {
            error!("Stale embedding regeneration failed: {err}");
        }

        let mut ticker = interval(DEFAULT_REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        info!("Feed refresh scheduler started (interval: {DEFAULT_REFRESH_INTERVAL:?})");
        loop {
            ticker.tick().await;
            if let Err(err) = articles::refresh_all_feeds().await {
                error!("Feed refresh failed: {err}");
            }
            // if let Err(err) = articles::regenerate_articles().await {
            //     error!("Article regeneration failed: {err}");
            // }
            if let Err(err) = database::cleanup_binned(7).await {
                error!("Cleanup failed: {err}");
            }
        }
    });
}
