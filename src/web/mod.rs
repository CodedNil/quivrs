mod article;
mod components;
mod sidebar;

use crate::shared::{
    Article, ArticleStatus, Category, Rating,
    server_functions::{get_all_item_ratings, get_articles, set_article_status, set_rating},
    theme::THEME_CSS_VARS,
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

const STATUS_LANES: [ArticleStatus; 3] = [
    ArticleStatus::Stored,
    ArticleStatus::New,
    ArticleStatus::Binned,
];

#[component]
fn AppHead() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap",
        }
        document::Style {
            "*, *::before, *::after {{ box-sizing: border-box; }}
            html {{
                min-height: 100%;
                background: var(--mantle);
            }}
            body {{
                {THEME_CSS_VARS}

                font-family: 'Inter', system-ui, sans-serif;
                margin: 0; padding: 0px;
                background:
                    radial-gradient(circle at 12% 0%, color-mix(in srgb, var(--accent) 18%, transparent), transparent 28rem),
                    radial-gradient(circle at 88% 12%, rgba(245, 197, 24, 0.08), transparent 24rem),
                    linear-gradient(135deg, var(--crust), var(--mantle) 45%, var(--base));
                color: var(--text);
                overflow: hidden;
            }}

            button, a {{
                font: inherit;
            }}

            button:focus-visible,
            a:focus-visible,
            #main-app-container:focus-visible {{
                outline: 2px solid var(--ring);
                outline-offset: 3px;
            }}

            ::selection {{
                background: color-mix(in srgb, var(--accent) 45%, transparent);
                color: var(--text);
            }}

            .material-icon {{
                display: inline-flex;
                align-items: center;
                justify-content: center;
                flex: 0 0 auto;
                line-height: 0;
                color: inherit;
            }}

            .material-icon svg {{
                width: 100%;
                height: 100%;
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
    ArticleDetail { tab: String, id: Uuid },
}

#[component]
fn TabHome(tab: String) -> Element {
    let articles = use_context::<Signal<Vec<Article>>>();
    let articles_loaded = use_context::<Signal<bool>>();
    let status = status_for_tab(&tab);
    let count = articles
        .read()
        .iter()
        .filter(|article| article.status == status)
        .count();
    let text = if !articles_loaded() {
        "Loading articles".to_string()
    } else if count == 0 {
        format!("No {tab} articles")
    } else {
        "Select an article to read".to_string()
    };

    rsx! {
        CenteredMessage { text }
    }
}

pub fn app() -> Element {
    let mut articles = use_context_provider(|| Signal::new(Vec::<Article>::new()));
    let mut articles_loaded = use_context_provider(|| Signal::new(false));
    let mut item_ratings = use_context_provider(|| Signal::new(HashMap::<String, Rating>::new()));

    use_resource(move || async move {
        if let Ok(data) = get_articles().await {
            articles.set(data);
        }
        articles_loaded.set(true);
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

    let (tab, selected_id) = route_parts(&route);
    let current_status = status_for_tab(&tab);

    // Filter dynamic IDs reactively
    let filtered_articles =
        use_memo(move || article_ids_for_status(&articles.read(), current_status));

    let onkeydown = {
        let tab = tab.clone();
        move |event: KeyboardEvent| {
            let Some(id) = selected_id else {
                return;
            };

            let navigate_delta = |delta: i32| {
                if let Some(target_id) = adjacent_article_id(&filtered_articles.read(), id, delta) {
                    navigator.push(Route::ArticleDetail {
                        tab: tab.clone(),
                        id: target_id,
                    });
                }
            };

            let scroll_content = |direction: f64| {
                if let Some(handle) = content_container_handle.read().as_ref() {
                    let amt = direction * 350.0;
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
            };

            match event.key() {
                Key::ArrowLeft => navigate_delta(-1),
                Key::ArrowRight => navigate_delta(1),
                Key::ArrowUp => scroll_content(-1.0),
                Key::ArrowDown => scroll_content(1.0),
                Key::Character(c) => match c.to_lowercase().as_str() {
                    "w" => scroll_content(-1.0),
                    "s" => scroll_content(1.0),
                    "a" => navigate_delta(-1),
                    "d" => navigate_delta(1),
                    "1" | "2" | "3" | "4" | "5" => {
                        let rating = match c.as_str() {
                            "1" => Rating::Hated,
                            "2" => Rating::Disliked,
                            "3" => Rating::Neutral,
                            "4" => Rating::Liked,
                            _ => Rating::Loved,
                        };
                        spawn(async move {
                            set_article_rating_local(articles, id, Some(rating));
                            let _ = set_rating(id, rating).await;
                        });
                    }
                    "r" | "t" => {
                        let status = if c == "r" {
                            ArticleStatus::Binned
                        } else {
                            ArticleStatus::Stored
                        };

                        let route = route_after_status_change(
                            tab.clone(),
                            &articles.read(),
                            current_status,
                            id,
                        );
                        navigator.push(route);

                        set_article_status_local(articles, id, status);
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
            main {
                id: "article-main",
                flex: "1",
                overflow: "hidden",
                padding: "10px",
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

fn status_for_tab(tab: &str) -> ArticleStatus {
    match tab {
        "stored" => ArticleStatus::Stored,
        "binned" => ArticleStatus::Binned,
        _ => ArticleStatus::New,
    }
}

fn route_parts(route: &Route) -> (String, Option<Uuid>) {
    match route {
        Route::ArticleDetail { tab, id } => (tab.clone(), Some(*id)),
        Route::TabHome { tab } => (tab.clone(), None),
    }
}

fn article_ids_for_status(articles: &[Article], status: ArticleStatus) -> Vec<Uuid> {
    let mut groups = BTreeMap::<Category, Vec<Uuid>>::new();
    for article in articles {
        if article.status == status {
            groups.entry(article.category).or_default().push(article.id);
        }
    }
    groups.into_values().flatten().collect()
}

fn adjacent_article_id(list: &[Uuid], id: Uuid, delta: i32) -> Option<Uuid> {
    if list.len() < 2 {
        return None;
    }

    let current = list.iter().position(|&item_id| item_id == id)?;
    let target = (current as i32 + delta).rem_euclid(list.len() as i32) as usize;
    (list[target] != id).then_some(list[target])
}

fn article_exists_in_tab(articles: &[Article], tab: &str, id: Uuid) -> bool {
    let status = status_for_tab(tab);
    articles
        .iter()
        .any(|article| article.id == id && article.status == status)
}

fn route_after_status_change(
    tab: String,
    articles: &[Article],
    current_status: ArticleStatus,
    id: Uuid,
) -> Route {
    let list = article_ids_for_status(articles, current_status);
    if let Some(pos) = list.iter().position(|&item_id| item_id == id)
        && list.len() > 1
    {
        let target_index = if pos + 1 < list.len() {
            pos + 1
        } else {
            pos.saturating_sub(1)
        };
        let target_id = list[target_index];
        if target_id != id {
            return Route::ArticleDetail { tab, id: target_id };
        }
    }

    Route::TabHome { tab }
}

fn set_article_status_local(mut articles: Signal<Vec<Article>>, id: Uuid, status: ArticleStatus) {
    if let Some(article) = articles.write().iter_mut().find(|a| a.id == id) {
        article.status = status;
    }
}

fn set_article_rating_local(mut articles: Signal<Vec<Article>>, id: Uuid, rating: Option<Rating>) {
    if let Some(article) = articles.write().iter_mut().find(|a| a.id == id) {
        article.rating = rating;
    }
}
