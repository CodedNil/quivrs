mod article;
mod components;
mod sidebar;

use crate::shared::{
    Article, Rating,
    server_functions::{get_all_item_ratings, get_user_articles},
};
use article::ArticleDetail;
use components::CenteredMessage;
use dioxus::prelude::*;
use sidebar::{SIDEBAR_STYLES, Sidebar};
use std::collections::HashMap;
use uuid::Uuid;

#[component]
fn AppHead() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap",
        }
        document::Style {
            "*, *::before, *::after {{ box-sizing: border-box; }}
            body {{
                --crust: #181926;
                --mantle: #1e2030;
                --base: #24273a;
                --surface0: #363a4f;
                --surface1: #494d64;
                --surface2: #5b6078;
                --overlay0: #6e738d;
                --overlay1: #8087a2;
                --overlay2: #939ab7;
                --text: #cad3f5;
                --subtext0: #a5adcb;
                --subtext1: #b8c0e0;
                --accent: #8aadf4;

                --crust-transparent: color-mix(in srgb, var(--crust) 30%, transparent);
                --mantle-transparent: color-mix(in srgb, var(--mantle) 30%, transparent);
                --base-transparent: color-mix(in srgb, var(--base) 30%, transparent);

                font-family: 'Inter', system-ui, sans-serif;
                margin: 0; padding: 0px;
                background-color: var(--mantle);
            }}
            {SIDEBAR_STYLES}"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[layout(MainLayout)]
    #[redirect("/", || Route::TabHome { tab: "new".to_string() })]
    #[route("/:tab")]
    TabHome { tab: String },
    #[route("/:tab/entry/:id")]
    ArticleEntry { tab: String, id: Uuid },
}

#[component]
fn TabHome(tab: String) -> Element {
    rsx! {
        CenteredMessage { text: "Select an article to read" }
    }
}

#[component]
fn ArticleEntry(tab: String, id: Uuid) -> Element {
    let articles = use_context::<Signal<Vec<Article>>>();
    let item_ratings = use_context::<Signal<HashMap<String, Rating>>>();

    rsx! {
        ArticleDetail { id, articles, item_ratings }
    }
}

pub fn app() -> Element {
    let mut articles: Signal<Vec<Article>> = use_signal(Vec::new);
    let mut item_ratings: Signal<HashMap<String, Rating>> = use_signal(HashMap::new);

    use_context_provider(|| articles);
    use_context_provider(|| item_ratings);

    use_resource(move || async move {
        if let Ok(data) = get_user_articles().await {
            *articles.write() = data;
        }
    });
    use_resource(move || async move {
        if let Ok(data) = get_all_item_ratings().await {
            *item_ratings.write() = data;
        }
    });

    rsx! {
        AppHead {}
        document::Title { "Quivrs" }
        Router::<Route> {}
    }
}

#[component]
fn MainLayout() -> Element {
    let route = use_route::<Route>();

    let (tab, selected_id) = match route {
        Route::ArticleEntry { tab, id } => (tab, Some(id)),
        Route::TabHome { tab } => (tab, None),
    };

    #[cfg(target_arch = "wasm32")]
    {
        let tab_for_effect = tab.clone();
        use_effect(move || {
            if let Some(target_id) = selected_id {
                let articles = use_context::<Signal<Vec<Article>>>();
                let tab = tab_for_effect.clone();
                spawn(async move {
                    use wasm_bindgen::JsCast;
                    let window = web_sys::window().unwrap();
                    let _ = gloo_timers::future::TimeoutFuture::new(100).await;

                    // Check if the article exists in the current tab's status
                    let current_status = match tab.as_str() {
                        "stored" => crate::shared::ArticleStatus::Stored,
                        "binned" => crate::shared::ArticleStatus::Binned,
                        _ => crate::shared::ArticleStatus::New,
                    };

                    let exists_in_tab = articles
                        .read()
                        .iter()
                        .any(|a| a.id == target_id && a.status == current_status);

                    if !exists_in_tab {
                        return;
                    }

                    let document = window.document().unwrap();
                    if let Some(art_el) = document
                        .get_element_by_id(&format!("article-{target_id}"))
                        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        art_el.scroll_into_view_with_scroll_into_view_options(
                            web_sys::ScrollIntoViewOptions::new()
                                .behavior(web_sys::ScrollBehavior::Instant)
                                .block(web_sys::ScrollLogicalPosition::Center),
                        );
                    }
                });
            }
        });
    }

    rsx! {
        div {
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            color: "var(--text)",
            Sidebar { tab, selected_id }
            main { flex: "1", overflow: "hidden", padding: "10px",

                div {
                    background_color: "var(--base)",
                    border_radius: "20px",
                    height: "100%",
                    overflow_y: "auto",
                    Outlet::<Route> {}
                }
            }
        }
    }
}
