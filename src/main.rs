mod feed;
mod llm_functions;

use anyhow::Result;
use axum::{Router, routing::get};
use feed::{init_storage, refresh_all_feeds, summarised_feed_handler};
use std::time::Duration;
use tokio::{
    net::TcpListener,
    time::{self, MissedTickBehavior},
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    init_storage().await?;

    tokio::spawn(async move {
        let mut ticker = time::interval(DEFAULT_REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        info!("Starting feed refresh scheduler (interval: {DEFAULT_REFRESH_INTERVAL:?})",);
        loop {
            ticker.tick().await;
            if let Err(err) = refresh_all_feeds().await {
                error!("Scheduled feed refresh failed: {err}");
            }
        }
    });

    let app = Router::new().route("/feeds/{id}", get(summarised_feed_handler));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT environment variable must be a valid port number");
    let listen_addr = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(&listen_addr).await?;
    info!("Listening on {}", listen_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
