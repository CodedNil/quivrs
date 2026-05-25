mod article;
mod components;
mod sidebar;

use crate::shared::{
    Article, ArticleStatus, Rating,
    server_functions::{get_all_item_ratings, get_user_articles, set_article_status, set_rating},
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
    let navigator = use_navigator();
    let mut articles = use_context::<Signal<Vec<Article>>>();
    let all_groups = use_memo(use_reactive!(|articles| {
        let mut groups = std::collections::BTreeMap::<
            ArticleStatus,
            std::collections::BTreeMap<crate::shared::Category, Vec<Uuid>>,
        >::new();
        for a in &articles() {
            groups
                .entry(a.status)
                .or_default()
                .entry(a.category)
                .or_default()
                .push(a.id);
        }
        groups
    }));

    let (tab, selected_id) = match route {
        Route::ArticleEntry { tab, id } => (tab, Some(id)),
        Route::TabHome { tab } => (tab, None),
    };

    let current_status = match tab.as_str() {
        "stored" => ArticleStatus::Stored,
        "binned" => ArticleStatus::Binned,
        _ => ArticleStatus::New,
    };

    let filtered_articles = use_memo(move || {
        let mut list = Vec::new();
        let status_groups = match all_groups.read().get(&current_status) {
            Some(g) => g.clone(),
            None => return list,
        };

        for ids in status_groups.values() {
            for &id in ids {
                list.push(id);
            }
        }
        list
    });

    let tab_for_sidebar = tab.clone();
    let onkeydown = {
        let tab = tab.clone();
        move |event: KeyboardEvent| {
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

                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            let window = web_sys::window().unwrap();
                            let document = window.document().unwrap();
                            if let Some(art_el) = document
                                .get_element_by_id(&format!("article-{target_id}"))
                                .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
                            {
                                art_el.scroll_into_view_with_scroll_into_view_options(
                                    web_sys::ScrollIntoViewOptions::new()
                                        .behavior(web_sys::ScrollBehavior::Smooth)
                                        .block(web_sys::ScrollLogicalPosition::Center),
                                );
                            }
                        }
                    }
                }
                Key::ArrowUp | Key::ArrowDown => {
                    #[cfg(target_arch = "wasm32")]
                    if let Some(el) = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.get_element_by_id("article-content-container"))
                    {
                        let amt = if event.key() == Key::ArrowUp {
                            -350.0
                        } else {
                            350.0
                        };
                        el.scroll_by_with_scroll_to_options(
                            web_sys::ScrollToOptions::new()
                                .top(amt)
                                .behavior(web_sys::ScrollBehavior::Smooth),
                        );
                    }
                }
                Key::Character(c) => match c.as_str() {
                    "1" | "2" | "3" | "4" | "5" => {
                        let rating = match c.as_str() {
                            "1" => Rating::Hated,
                            "2" => Rating::Disliked,
                            "3" => Rating::Neutral,
                            "4" => Rating::Liked,
                            "5" => Rating::Loved,
                            _ => unreachable!(),
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
                        let tab = tab.clone();
                        spawn(async move {
                            if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                                a.status = status;
                            }
                            let _ = set_article_status(id, status).await;

                            let list = filtered_articles.read();
                            if let Some(p) = list.iter().position(|&item_id| item_id == id) {
                                let next = list
                                    .get(p + 1)
                                    .or_else(|| if p > 0 { list.get(p - 1) } else { None });
                                if let Some(&a_id) = next {
                                    navigator.push(Route::ArticleEntry { tab, id: a_id });
                                } else {
                                    navigator.push(Route::TabHome { tab });
                                }
                            }
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        }
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
                                .behavior(web_sys::ScrollBehavior::Smooth)
                                .block(web_sys::ScrollLogicalPosition::Center),
                        );
                    }
                });
            }
        });
    }

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
            Sidebar { tab: tab_for_sidebar, selected_id }
            main { flex: "1", overflow: "hidden", padding: "10px",

                div {
                    id: "article-content-container",
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
