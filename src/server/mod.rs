pub mod articles;
pub mod database;
mod embeddings;
mod llm_functions;
mod parse_feed;

use reqwest::Client;
use std::{sync::LazyLock, time::Duration};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{error, info};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Quivrs/1.0")
        .build()
        .expect("Failed to build HTTP client")
});

pub fn start() {
    tokio::spawn(async move {
        let mut ticker = interval(DEFAULT_REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        info!("Feed refresh scheduler started (interval: {DEFAULT_REFRESH_INTERVAL:?})");
        loop {
            ticker.tick().await;
            if let Err(err) = articles::refresh_all_feeds().await {
                error!("Feed refresh failed: {err}");
            }
            if let Err(err) = articles::regenerate_articles().await {
                error!("Article regeneration failed: {err}");
            }
        }
    });
}
