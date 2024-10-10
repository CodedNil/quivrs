#![allow(
    non_snake_case,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::module_name_repetitions,
    clippy::ref_option_ref
)]

use dioxus::prelude::*;
use dioxus_logger::tracing;

#[cfg(feature = "server")]
mod server;

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");

    #[cfg(feature = "web")]
    dioxus_web::launch::launch_cfg(app, dioxus_web::Config::new().hydrate(true));

    #[cfg(feature = "server")]
    {
        use axum::routing::Router;
        use axum_session::SessionConfig;
        use axum_session::SessionStore;
        use axum_session_auth::AuthConfig;
        use axum_session_sqlx::SessionSqlitePool;

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let pool = server::auth::connect_to_database().await;

                // Configure the authentication layer
                let session_config = SessionConfig::default().with_table_name("test_table");
                let auth_config = AuthConfig::<i64>::default().with_anonymous_user_id(Some(1));
                let session_store = SessionStore::<SessionSqlitePool>::new(
                    Some(pool.clone().into()),
                    session_config,
                )
                .await
                .unwrap();

                server::auth::User::create_user_tables(&pool).await;

                // Spawn server::init to run in parallel
                let init_handle = tokio::spawn(async {
                    server::init().await;
                });

                // Build our application with some routes
                let app = Router::new()
                    .serve_dioxus_application(ServeConfig::builder().build(), || {
                        VirtualDom::new(app)
                    })
                    .await
                    .layer(
                        axum_session_auth::AuthSessionLayer::<
                            server::auth::User,
                            i64,
                            SessionSqlitePool,
                            sqlx::SqlitePool,
                        >::new(Some(pool))
                        .with_config(auth_config),
                    )
                    .layer(axum_session::SessionLayer::new(session_store));

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
    let mut user_name = use_signal(|| "?".to_string());
    let mut permissions = use_signal(|| "?".to_string());

    rsx! {
        div {
            button { onclick: move |_| {
                    async move {
                        login().await.unwrap();
                    }
                },
                "Login Test User"
            }
        }
        div {
            button {
                onclick: move |_| async move {
                    if let Ok(data) = get_user_name().await {
                        user_name.set(data);
                    }
                },
                "Get User Name"
            }
            "User name: {user_name}"
        }
        div {
            button {
                onclick: move |_| async move {
                    if let Ok(data) = get_permissions().await {
                        permissions.set(data);
                    }
                },
                "Get Permissions"
            }
            "Permissions: {permissions}"
        }
    }
}

#[server(GetUserName)]
pub async fn get_user_name() -> Result<String, ServerFnError> {
    let session: server::auth::Session = extract().await?;
    Ok(session.0.current_user.unwrap().username)
}

#[server(Login)]
pub async fn login() -> Result<(), ServerFnError> {
    let auth: server::auth::Session = extract().await?;
    auth.login_user(2);
    Ok(())
}

#[server(Permissions)]
pub async fn get_permissions() -> Result<String, ServerFnError> {
    let method: axum::http::Method = extract().await?;
    let auth: server::auth::Session = extract().await?;
    let current_user = auth.current_user.clone().unwrap_or_default();

    // lets check permissions only and not worry about if they are anon or not
    if !axum_session_auth::Auth::<server::auth::User, i64, sqlx::SqlitePool>::build(
        [axum::http::Method::POST],
        false,
    )
    .requires(axum_session_auth::Rights::any([
        axum_session_auth::Rights::permission("Category::View"),
        axum_session_auth::Rights::permission("Admin::View"),
    ]))
    .validate(&current_user, &method, None)
    .await
    {
        return Ok(format!(
            "User {}, Does not have permissions needed to view this page please login",
            current_user.username
        ));
    }

    Ok(format!(
        "User has Permissions needed. Here are the Users permissions: {:?}",
        current_user.permissions
    ))
}
