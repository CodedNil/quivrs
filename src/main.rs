#![allow(
    non_snake_case,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::module_name_repetitions,
    clippy::ref_option_ref
)]

mod common;

#[cfg(feature = "server")]
mod server;

use arrayvec::ArrayString;
use common::TOKEN_LENGTH;
use dioxus::prelude::*;
use dioxus_logger::tracing;
use dioxus_sdk::storage::{use_synced_storage, LocalStorage};

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    dioxus_sdk::storage::set_dir!();

    #[cfg(feature = "web")]
    dioxus_web::launch::launch_cfg(app, dioxus_web::Config::new().hydrate(true));

    #[cfg(feature = "server")]
    {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                // Spawn server::init to run in parallel
                let init_handle = tokio::spawn(async {
                    server::init().await;
                });

                // Build our application with some routes
                let app = axum::routing::Router::new()
                    .serve_dioxus_application(ServeConfig::builder().build(), || {
                        VirtualDom::new(app)
                    })
                    .await;

                let listener = tokio::net::TcpListener::bind(&std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    3000,
                )))
                .await
                .unwrap();

                axum::serve(listener, app.into_make_service())
                    .await
                    .unwrap();

                let _ = init_handle.await;
            });
    }
}

fn app() -> Element {
    let mut token = use_synced_storage::<LocalStorage, ArrayString<TOKEN_LENGTH>>(
        "auth_token".to_string(),
        ArrayString::<TOKEN_LENGTH>::new,
    );

    rsx! {
        div {
            if !token.peek().is_empty() {
                "Logged in!"
            } else {
                form {
                    onsubmit: move |form_data| async move {
                        if let (Some(user), Some(pass)) = (
                            form_data.values().get("username").and_then(|m| m.first()),
                            form_data.values().get("password").and_then(|m| m.first()),
                        ) {
                            if let Ok(data) = login(user.to_string(), pass.to_string()).await {
                                token.set(data);
                            }
                        }
                    },
                    input {
                        r#type: "text",
                        placeholder: "Username",
                        name: "username",
                        maxlength: 20,
                        pattern: "^[a-zA-Z0-9 ]+$"
                    }
                    input {
                        r#type: "text",
                        placeholder: "Password",
                        name: "password",
                        maxlength: 30
                    }
                    button { r#type: "submit", "Login" }
                }
            }
        }
    }
}

#[server(Login)]
pub async fn login(
    username: String,
    password: String,
) -> Result<ArrayString<TOKEN_LENGTH>, ServerFnError> {
    if let Ok((_, token)) = server::auth::login(username, password).await {
        return Ok(token);
    }
    Err(ServerFnError::new("Invalid credentials"))
}
