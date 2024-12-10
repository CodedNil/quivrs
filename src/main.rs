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

use dioxus::prelude::*;
use dioxus_logger::tracing;
use dioxus_sdk::storage::{use_synced_storage, LocalStorage};

#[cfg(feature = "server")]
const PORT: u16 = 4895;

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
                let router = axum::routing::Router::new()
                    .serve_dioxus_application(ServeConfig::new().unwrap(), app)
                    .into_make_service();

                let listener = tokio::net::TcpListener::bind(&std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    PORT,
                )))
                .await
                .unwrap();

                axum::serve(listener, router).await.unwrap();

                let _ = init_handle.await;
            });
    }
}

fn app() -> Element {
    let mut token =
        use_synced_storage::<LocalStorage, String>("auth_token".to_string(), String::new);

    tracing::info!(
        "Starting app with token: {} {}",
        token.peek(),
        !token.peek().is_empty()
    );
    rsx! {
        div { class: "container",
            if token.peek().is_empty() {
                LoginForm { on_login: move |data| {
                    token.set(data);
                } }
            } else {
                div { "Logged in!" }
            }
        }
    }
}

#[component]
fn LoginForm(on_login: EventHandler<String>) -> Element {
    rsx! {
        div { class: "login-form-container",
            form {
                style: "display: flex; flex-direction: column;",
                onsubmit: move |form_data| async move {
                    if let (Some(user), Some(pass)) = (
                        form_data.values().get("username").and_then(|m| m.first()),
                        form_data.values().get("password").and_then(|m| m.first()),
                    ) {
                        if let Ok(data) = login(user.to_string(), pass.to_string()).await {
                            on_login.call(data);
                        }
                    }
                },
                input {
                    class: "input-field",
                    r#type: "text",
                    placeholder: "Username",
                    name: "username",
                    maxlength: 20,
                    pattern: "^[a-zA-Z0-9 ]+$"
                }
                input {
                    class: "input-field",
                    r#type: "password",
                    placeholder: "Password",
                    name: "password",
                    maxlength: 30
                }
                button { class: "submit-button", r#type: "submit", "Login" }
            }
        }
    }
}

#[server(Login)]
pub async fn login(username: String, password: String) -> Result<String, ServerFnError> {
    match server::auth::login(username, password).await {
        Ok((_, token)) => Ok(token),
        Err(e) => {
            tracing::error!("Failed to login {}", e);
            Err(ServerFnError::new("Invalid credentials"))
        }
    }
}
