mod shared;

#[cfg(feature = "server")]
mod server;

mod web;

fn main() {
    #[cfg(feature = "web")]
    dioxus::launch(web::app);

    #[cfg(feature = "server")]
    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(16 * 1024 * 1024)
        .enable_all()
        .build()
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

    if !tracing::dispatcher::has_been_set() {
        dioxus::logger::init(tracing::Level::INFO).unwrap();
    }

    server::start();

    let app = axum::Router::<FullstackState>::new()
        .serve_dioxus_application(ServeConfig::new(), web::app)
        .layer(CompressionLayer::new());

    let addr = dioxus::cli_config::fullstack_address_or_localhost();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
