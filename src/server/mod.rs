mod embeddings;
pub mod feed;
mod llm_functions;

use reqwest::Client;
use std::{sync::LazyLock, time::Duration};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{error, info};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

pub fn start() {
    tokio::spawn(async move {
        let mut ticker = interval(DEFAULT_REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        info!("Feed refresh scheduler started (interval: {DEFAULT_REFRESH_INTERVAL:?})");
        loop {
            ticker.tick().await;
            if let Err(err) = feed::refresh_all_feeds().await {
                error!("Feed refresh failed: {err}");
            }
        }
    });
}
