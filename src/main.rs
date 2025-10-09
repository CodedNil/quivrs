mod feed;
mod llm_functions;

use anyhow::Result;
use axum::{routing::get, Router};
use dotenvy::dotenv;
use feed::{refresh_all_feeds, summarised_feed_handler};
use std::time::Duration;
use tokio::{
    net::TcpListener,
    time::{self, MissedTickBehavior},
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:3000";
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tokio::spawn(async move {
        let mut ticker = time::interval(DEFAULT_REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        info!("Starting feed refresh scheduler (interval: {DEFAULT_REFRESH_INTERVAL:?})",);
        loop {
            if let Err(err) = refresh_all_feeds().await {
                error!("Scheduled feed refresh failed: {err}");
            }
            ticker.tick().await;
        }
    });

    let app = Router::new().route("/feeds/{id}", get(summarised_feed_handler));

    let listener = TcpListener::bind(DEFAULT_LISTEN_ADDR).await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
