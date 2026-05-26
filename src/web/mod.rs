mod article;
mod components;
mod sidebar;

use crate::shared::{
    Article, ArticleStatus, Category, Rating,
    server_functions::{get_all_item_ratings, get_user_articles, set_article_status, set_rating},
};
use article::ArticleDetail;
use components::CenteredMessage;
use dioxus::{html::geometry::euclid::Vector2D, prelude::*};
use sidebar::{SIDEBAR_STYLES, Sidebar};
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};
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
    let mut articles = use_context_provider(|| Signal::new(Vec::<Article>::new()));
    let mut item_ratings = use_context_provider(|| Signal::new(HashMap::<String, Rating>::new()));

    use_resource(move || async move {
        if let Ok(data) = get_user_articles().await {
            articles.set(data);
        }
    });
    use_resource(move || async move {
        if let Ok(data) = get_all_item_ratings().await {
            item_ratings.set(data);
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
    let navigator = use_navigator();
    let articles = use_context::<Signal<Vec<Article>>>();

    // Store handle for the content container element to control scrolling smoothly
    let mut content_container_handle = use_signal(|| None::<Rc<MountedData>>);

    let (tab, selected_id) = match route {
        Route::ArticleEntry { tab, id } => (tab, Some(id)),
        Route::TabHome { tab } => (tab, None),
    };

    let current_status = match tab.as_str() {
        "stored" => ArticleStatus::Stored,
        "binned" => ArticleStatus::Binned,
        _ => ArticleStatus::New,
    };

    // Filter dynamic IDs reactively
    let filtered_articles = use_memo(move || {
        let mut groups = BTreeMap::<Category, Vec<Uuid>>::new();
        for a in articles.read().iter() {
            if a.status == current_status {
                groups.entry(a.category).or_default().push(a.id);
            }
        }
        groups.into_values().flatten().collect::<Vec<Uuid>>()
    });

    let onkeydown = {
        let tab = tab.clone();
        move |event: KeyboardEvent| {
            let mut articles = articles;
            let Some(id) = selected_id else {
                return;
            };
            let list = filtered_articles.read();
            let pos = list.iter().position(|&item_id| item_id == id);

            match event.key() {
                Key::ArrowLeft | Key::ArrowRight => {
                    if let (Some(p), false) = (pos, list.is_empty()) {
                        let delta = if event.key() == Key::ArrowLeft {
                            list.len() - 1
                        } else {
                            1
                        };
                        let target_id = list[(p + delta) % list.len()];
                        navigator.push(Route::ArticleEntry {
                            tab: tab.clone(),
                            id: target_id,
                        });
                    }
                }
                Key::ArrowUp | Key::ArrowDown => {
                    if let Some(handle) = content_container_handle.read().as_ref() {
                        let amt = if event.key() == Key::ArrowUp {
                            -1.0
                        } else {
                            1.0
                        } * 350.0;
                        let handle = handle.clone();
                        spawn(async move {
                            if let Ok(scroll_offset) = handle.get_scroll_offset().await {
                                let _ = handle
                                    .scroll(
                                        Vector2D::new(scroll_offset.x, scroll_offset.y + amt),
                                        ScrollBehavior::Smooth,
                                    )
                                    .await;
                            }
                        });
                    }
                }
                Key::Character(c) => match c.as_str() {
                    "1" | "2" | "3" | "4" | "5" => {
                        let rating = match c.as_str() {
                            "1" => Rating::Hated,
                            "2" => Rating::Disliked,
                            "3" => Rating::Neutral,
                            "4" => Rating::Liked,
                            _ => Rating::Loved,
                        };
                        spawn(async move {
                            if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                                a.rating = Some(rating);
                            }
                            let _ = set_rating(id, rating).await;
                        });
                    }
                    "r" | "t" => {
                        let status = if c == "r" {
                            ArticleStatus::Binned
                        } else {
                            ArticleStatus::Stored
                        };

                        // Move to next article
                        if let (Some(p), false) = (pos, list.is_empty()) {
                            let target_id = list[(p + 1) % list.len()];
                            navigator.push(Route::ArticleEntry {
                                tab: tab.clone(),
                                id: target_id,
                            });
                        }

                        // Update article status
                        if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                            a.status = status;
                        }
                        spawn(async move {
                            let _ = set_article_status(id, status).await;
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    };

    rsx! {
        div {
            id: "main-app-container",
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            color: "var(--text)",
            tabindex: "0",
            autofocus: true,
            onkeydown,
            Sidebar { tab, selected_id }
            main { flex: "1", overflow: "hidden", padding: "10px",
                div {
                    id: "article-content-container",
                    background_color: "var(--base)",
                    border_radius: "20px",
                    height: "100%",
                    overflow_y: "auto",
                    onmounted: move |cx| *content_container_handle.write() = Some(cx.data()),
                    Outlet::<Route> {}
                }
            }
        }
    }
}
