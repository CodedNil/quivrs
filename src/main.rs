#![allow(non_snake_case, clippy::similar_names, clippy::cast_precision_loss)]

use dioxus::prelude::*;
use dioxus_logger::tracing;

#[cfg(feature = "server")]
mod server;

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    tracing::info!("starting app");

    launch(App);
}

#[component]
fn App() -> Element {
    let mut meaning = use_signal(|| None);

    rsx! {
        h1 { "Meaning of life: {meaning:?}" }
        button {
            onclick: move |_| async move {
                if let Ok(data) = get_meaning("life the universe and everything".into()).await {
                    meaning.set(data);
                }
            },
            "Run a server function"
        }
    }
}

#[server]
async fn get_meaning(of: String) -> Result<Option<u32>, ServerFnError> {
    if let Err(e) = server::embeddings::candle_test().await {
        return Err(ServerFnError::ServerError(e.to_string()));
    }
    Ok(of.contains("life").then_some(42))
}
