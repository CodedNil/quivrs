mod article;
mod components;
mod sidebar;

use article::ArticleDetail;
use sidebar::Sidebar;

use crate::shared::{
    ArticleData, ArticleStatus, Rating,
    server_functions::{get_all_item_ratings, get_user_articles},
};
use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

mod style {
    pub const RADIUS_CARD: &str = "0.625rem";
    pub const RADIUS_GROUP: &str = "0.875rem";
}

const fn rating_color(r: Rating) -> &'static str {
    match r {
        Rating::Hated => "#873535",
        Rating::Disliked => "#A3674E",
        Rating::Neutral => "#494d64",
        Rating::Liked => "#5D7D31",
        Rating::Loved => "#3E6E2F",
    }
}

#[component]
fn AppHead() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap",
        }
        // <https://raw.githubusercontent.com/chriskempson/base16/refs/heads/main/styling.md>
        // Colours base00 to base07 are typically variations of a shade and run from darkest to lightest. These colours are used for foreground and background, status bars, line highlighting and such. colours base08 to base0F are typically individual colours used for types, operators, names and variables. In order to create a dark theme, colours base00 to base07 should span from dark to light. For a light theme, these colours should span from light to dark.
        document::Style {
            "body {{
                --base00: #24273a; --base01: #1e2030; --base02: #363a4f; --base03: #494d64;
                --base04: #5b6078; --base05: #cad3f5; --base06: #f4dbd6; --base07: #b7bdf8;
                --base08: #ed8796; --base09: #f5a97f; --base0a: #eed49f; --base0b: #a6da95;
                --base0c: #8bd5ca; --base0d: #8aadf4; --base0e: #c6a0f6; --base0f: #f0c6c6;
                font-family: 'Inter', system-ui, sans-serif;
                margin: 0; padding: 0;
            }}
            ::selection {{
                background-color: color-mix(in srgb, var(--base0d) 40%, transparent);
                color: var(--base05);
            }}"
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
    Article { tab: String, id: Uuid },
}

#[component]
fn TabHome(tab: String) -> Element {
    rsx! {
        div {
            display: "flex",
            align_items: "center",
            justify_content: "center",
            height: "100%",
            color: "var(--base04)",
            font_size: "0.875rem",
            "Select an article to read"
        }
    }
}

#[component]
fn Article(tab: String, id: Uuid) -> Element {
    let articles = use_context::<Signal<Vec<ArticleData>>>();
    let item_ratings = use_context::<Signal<HashMap<String, Rating>>>();

    let found = articles
        .read()
        .iter()
        .find(|a| a.id == id)
        .map(|a| (a.status, a.rating, a.article.clone()));

    match found {
        Some((status, rating, art)) => rsx! {
            ArticleDetail {
                article: art,
                status,
                rating,
                articles,
                item_ratings,
            }
        },
        None => rsx! {
            div {
                display: "flex",
                align_items: "center",
                justify_content: "center",
                height: "100%",
                color: "var(--base04)",
                font_size: "0.875rem",
                "Article not found"
            }
        },
    }
}

pub fn app() -> Element {
    let mut articles: Signal<Vec<ArticleData>> = use_signal(Vec::new);
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
        Route::Article { tab, id } => (tab, Some(id)),
        Route::TabHome { tab } => (tab, None),
    };
    let active_status = match tab.as_str() {
        "stored" => ArticleStatus::Stored,
        "binned" => ArticleStatus::Binned,
        _ => ArticleStatus::New,
    };

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Some(target_id) = selected_id {
            spawn(async move {
                use wasm_bindgen::JsCast;
                let window = web_sys::window().unwrap();
                let _ = gloo_timers::future::TimeoutFuture::new(100).await;
                if let Some(el) = window
                    .document()
                    .unwrap()
                    .get_element_by_id(&format!("article-{target_id}"))
                    .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    el.scroll_into_view();
                }
            });
        }
    });

    rsx! {
        div {
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            background_color: "var(--base00)",
            color: "var(--base05)",
            Sidebar { tab, selected_id, active_status }
            main { flex: "1", overflow_y: "auto", Outlet::<Route> {} }
        }
    }
}
