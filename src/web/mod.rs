mod article;
mod components;
mod sidebar;

use crate::shared::{
    ArticleData, Rating,
    server_functions::{get_all_item_ratings, get_user_articles},
};
use article::ArticleDetail;
use dioxus::prelude::*;
use sidebar::{SIDEBAR_STYLES, Sidebar};
use std::collections::HashMap;
use uuid::Uuid;

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
            color: "var(--subtext0)",
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
                color: "var(--subtext0)",
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

    // Scroll to the selected article on mount
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Some(target_id) = selected_id {
            spawn(async move {
                use wasm_bindgen::JsCast;
                let window = web_sys::window().unwrap();
                let _ = gloo_timers::future::TimeoutFuture::new(100).await;
                if let (Some(scroll_el), Some(art_el)) = (
                    window
                        .document()
                        .unwrap()
                        .get_element_by_id("article-scroll-container")
                        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok()),
                    window
                        .document()
                        .unwrap()
                        .get_element_by_id(&format!("article-{target_id}"))
                        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok()),
                ) {
                    let sr = scroll_el.get_bounding_client_rect();
                    let ar = art_el.get_bounding_client_rect();
                    let target_top = scroll_el.scroll_top() as f64 + ar.top() - sr.top();
                    let final_top = (target_top - f64::from(sidebar::ARTICLE_GAP_PX)).max(0.0);

                    scroll_el.scroll_to_with_scroll_to_options(
                        web_sys::ScrollToOptions::new()
                            .top(final_top)
                            .behavior(web_sys::ScrollBehavior::Instant),
                    );
                }
            });
        }
    });

    rsx! {
        div {
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            color: "var(--text)",
            Sidebar { tab, selected_id }
            main { flex: "1", overflow_y: "auto", Outlet::<Route> {} }
        }
    }
}
