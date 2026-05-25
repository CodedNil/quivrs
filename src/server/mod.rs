pub mod articles;
pub mod database;
mod embeddings;
mod llm_functions;
pub mod parsers;

use axum::http::HeaderValue;
use reqwest::{Client, header};
use std::{sync::LazyLock, time::Duration};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{error, info};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_mins(15);
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
        ),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-GB,en;q=0.9"),
    );
    headers.insert(
        header::UPGRADE_INSECURE_REQUESTS,
        HeaderValue::from_static("1"),
    );
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("none"));
    headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
    headers.insert(
        "Sec-CH-UA",
        HeaderValue::from_static(
            r#""Chromium";v="136", "Google Chrome";v="136", "Not_A Brand";v="8""#,
        ),
    );
    headers.insert("Sec-CH-UA-Mobile", HeaderValue::from_static("?0"));
    headers.insert(
        "Sec-CH-UA-Platform",
        HeaderValue::from_static(r#""Windows""#),
    );
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .default_headers(headers)
        .cookie_store(true)
        .build()
        .expect("Failed to build HTTP client")
});

pub fn start() {
    assert!(
        std::env::var("OPENROUTER").is_ok(),
        "OPENROUTER environment variable not set."
    );

    tokio::spawn(async move {
        if let Err(err) = database::init().await {
            error!("Database initialisation failed: {err}");
            return;
        }

        let mut ticker = interval(DEFAULT_REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        info!("Feed refresh scheduler started (interval: {DEFAULT_REFRESH_INTERVAL:?})");
        loop {
            ticker.tick().await;
            if let Err(e) = database::maintenance_embeddings().await {
                error!("Embedding maintenance failed: {e}");
            }
            if let Err(err) = articles::refresh_all_feeds().await {
                error!("Feed refresh failed: {err}");
            }
            if let Err(err) = articles::promote_articles().await {
                error!("Article promotion failed: {err}");
            }
        }
    });
}
