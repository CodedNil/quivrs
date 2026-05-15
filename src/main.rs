#[cfg(feature = "server")]
mod feed;
#[cfg(feature = "server")]
mod llm_functions;
mod web;

use std::time::Duration;
use tracing::{error, info};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);

fn main() {
    #[cfg(feature = "web")]
    dioxus::launch(web::app);

    #[cfg(feature = "server")]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move { server_run().await });
}

#[cfg(feature = "server")]
async fn server_run() {
    use crate::feed::refresh_all_feeds;
    use dioxus::{
        prelude::dioxus_server::{FullstackState, ServeConfig},
        server::DioxusRouterExt,
    };
    use tokio::time::{MissedTickBehavior, interval};
    use tower_http::compression::CompressionLayer;

    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    tokio::spawn(async move {
        let mut ticker = interval(DEFAULT_REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        info!("Starting feed refresh scheduler (interval: {DEFAULT_REFRESH_INTERVAL:?})",);
        loop {
            ticker.tick().await;
            if let Err(err) = refresh_all_feeds().await {
                error!("Scheduled feed refresh failed: {err}");
            }
        }
    });

    let app = axum::Router::<FullstackState>::new()
        .serve_dioxus_application(ServeConfig::new(), web::app)
        .layer(CompressionLayer::new());

    let addr = dioxus::cli_config::fullstack_address_or_localhost();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
