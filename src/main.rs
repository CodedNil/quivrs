#[cfg(feature = "server")]
mod server;
mod shared;
mod web;

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
    use dioxus::{
        prelude::dioxus_server::{FullstackState, ServeConfig},
        server::DioxusRouterExt,
    };
    use tower_http::compression::CompressionLayer;

    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    dioxus::logger::initialize_default();

    server::start();

    let app = axum::Router::<FullstackState>::new()
        .serve_dioxus_application(ServeConfig::new(), web::app)
        .layer(CompressionLayer::new());

    let addr = dioxus::cli_config::fullstack_address_or_localhost();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
