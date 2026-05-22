use super::components::RatingPill;
use crate::shared::{
    ArticleData, ArticleStatus, Rating, StoredArticle,
    server_functions::{regenerate_article, set_article_status, set_rating},
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape, icons::fa_solid_icons};
use std::collections::HashMap;
use uuid::Uuid;

#[component]
pub fn ArticleDetail(
    article: StoredArticle,
    status: ArticleStatus,
    rating: Option<Rating>,
    articles: Signal<Vec<ArticleData>>,
    item_ratings: Signal<HashMap<String, Rating>>,
) -> Element {
    let id = article.id;
    let title = article.display_title();
    let thumbnail = article.thumbnail_image();

    rsx! {
        div {
            position: "relative",
            width: "100%",
            background_color: "var(--base)",
            border_radius: "20px 20px 0 0",
            overflow: "clip",

            if let Some(url) = thumbnail.as_ref() {
                // Hero header image
                div {
                    position: "absolute",
                    top: "0",
                    left: "0",
                    right: "0",
                    height: "26rem",
                    z_index: "0",
                    img {
                        src: "{url}",
                        width: "100%",
                        height: "100%",
                        object_fit: "cover",
                        display: "block",
                    }
                    // Bottom fade to content
                    div {
                        position: "absolute",
                        bottom: "0",
                        left: "0",
                        right: "0",
                        height: "100%",
                        background: "linear-gradient(to bottom,
                            rgba(24, 24, 37, 0) 0%,
                            rgba(24, 24, 37, 0.1) 10%,
                            rgba(24, 24, 37, 0.4) 30%,
                            rgba(24, 24, 37, 0.8) 60%,
                            var(--base) 85%,
                            var(--base) 100%),
                            linear-gradient(to bottom, transparent 0%, var(--base) 100%)",
                    }
                }

                // Background blurred image for the entire article
                div {
                    position: "absolute",
                    top: "-5%",
                    left: "-5%",
                    right: "-5%",
                    bottom: "-5%",
                    z_index: "1",
                    img {
                        src: "{url}",
                        width: "110%",
                        height: "110%",
                        object_fit: "fill",
                        filter: "blur(80px) saturate(3)",
                        opacity: "0.15",
                        display: "block",
                    }
                    // Noise overlay to prevent banding
                    div {
                        position: "absolute",
                        top: "0",
                        left: "0",
                        right: "0",
                        bottom: "0",
                        opacity: "0.1",
                        pointer_events: "none",
                        background_image: "url('{asset!(\"/assets/bluenoise.png\")}')",
                        background_repeat: "repeat",
                        background_size: "256px",
                        style: "mix-blend-mode: multiply;",
                    }
                }
            }

            div {
                position: "relative",
                z_index: "2",
                padding: if thumbnail.is_some() { "16rem 2rem 5rem" } else { "1.5rem 2rem 5rem" },

                div {
                    width: "100%",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",

                    h1 {
                        font_size: "1.625rem",
                        font_weight: "800",
                        line_height: "1.2",
                        color: "var(--text)",
                        margin_bottom: "0.75rem",
                        letter_spacing: "-0.01em",

                        // DEBUG click to reset the article generation
                        onclick: move |_| async move {
                            let _ = regenerate_article(id).await;
                        },

                        "{title}"
                    }

                    div {
                        display: "flex",
                        align_items: "center",
                        gap: "2rem",
                        margin_bottom: "2rem",

                        div {
                            display: "flex",
                            align_items: "center",
                            gap: "0.375rem",
                            if status != ArticleStatus::Stored {
                                ActionBtn {
                                    icon: fa_solid_icons::FaBookmark,
                                    title: "Save to Read Later",
                                    color: "var(--accent)",
                                    onclick: move |_| async move {
                                        let _ = set_article_status(id, ArticleStatus::Stored).await;
                                        if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                                            a.status = ArticleStatus::Stored;
                                        }
                                    },
                                }
                            }
                            if status != ArticleStatus::Binned {
                                ActionBtn {
                                    icon: fa_solid_icons::FaTrash,
                                    title: "Move to Bin",
                                    color: "var(--overlay0)",
                                    onclick: move |_| async move {
                                        let _ = set_article_status(id, ArticleStatus::Binned).await;
                                        if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                                            a.status = ArticleStatus::Binned;
                                        }
                                    },
                                }
                            }
                        }
                        span { font_size: "0.7rem", color: "var(--subtext0)",
                            {article.published.format("%b %d, %Y %H:%M UTC").to_string()}
                        }
                        StarRating { current: rating, id, articles }
                        div {
                            display: "flex",
                            align_items: "center",
                            gap: "0.375rem",
                            RatingPill {
                                label: article.category.to_string(),
                                item_key: format!("category:{}", article.category),
                                item_ratings,
                            }
                            for source in &article.sources {
                                RatingPill {
                                    key: "{source.url}",
                                    label: source.source.clone(),
                                    item_key: format!("source:{}", source.source),
                                    item_ratings,
                                    url: Some(source.url.clone()),
                                }
                            }
                        }
                    }
                }

                if let Some(entry) = &article.entry {
                    div {
                        display: "flex",
                        gap: "2.5rem",
                        align_items: "flex-start",
                        justify_content: "center",
                        flex_wrap: "wrap",
                        style { "{include_str!(\"article.css\")}" }

                        div {
                            class: "article-content",
                            display: "flex",
                            flex: "1",
                            flex_direction: "column",
                            gap: "1.5rem",
                            color: "var(--text)",
                            line_height: "1.8",
                            font_size: "1rem",
                            margin_left: "auto",
                            margin_right: "auto",
                            width: "44rem",
                            dangerous_inner_html: "{entry.content}",
                        }

                        if let Some(sidebar) = &entry.sidebar {
                            div {
                                class: "article-sidebar",
                                width: "20rem",
                                height: "fit-content",
                                flex_shrink: "0",
                                background: "var(--mantle-transparent)",
                                border_radius: "16px",
                                padding: "1.25rem",
                                font_size: "0.875rem",
                                position: "sticky",
                                top: "2rem",
                                dangerous_inner_html: "{sidebar}",
                            }
                        }
                    }
                } else {
                    p {
                        color: "var(--subtext0)",
                        font_size: "0.875rem",
                        font_style: "italic",
                        margin_bottom: "1rem",
                        "Generating summary…"
                    }
                    if let Some(s) = article.sources.first() {
                        div {
                            font_size: "0.825rem",
                            color: "var(--subtext0)",
                            line_height: "1.65",
                            white_space: "pre-wrap",
                            "{s.content}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ActionBtn<T: IconShape + Clone + PartialEq + 'static>(
    icon: T,
    title: String,
    color: &'static str,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let mut hovered = use_signal(|| false);
    rsx! {
        button {
            padding: "0.375rem",
            border_radius: "0.375rem",
            border: "none",
            background_color: if hovered() { "var(--base)" } else { "transparent" },
            color: if hovered() { color } else { "var(--subtext0)" },
            cursor: "pointer",
            transition: "background-color 0.15s, color 0.15s",
            display: "flex",
            align_items: "center",
            justify_content: "center",
            title,
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            onclick,
            Icon { icon, width: 15, height: 15 }
        }
    }
}

#[component]
fn StarRating(
    current: Option<Rating>,
    id: Uuid,
    mut articles: Signal<Vec<ArticleData>>,
) -> Element {
    const RATINGS: [Rating; 5] = [
        Rating::Hated,
        Rating::Disliked,
        Rating::Neutral,
        Rating::Liked,
        Rating::Loved,
    ];
    let current_idx = current.and_then(|r| RATINGS.iter().position(|&x| x == r));
    let mut hover_idx: Signal<Option<usize>> = use_signal(|| None);
    let fill_to = hover_idx().or(current_idx);

    rsx! {
        div { display: "flex", align_items: "center", gap: "0.05rem",
            for (i, &this_rating) in RATINGS.iter().enumerate() {
                {
                    let filled = fill_to.is_some_and(|h| i <= h);
                    let is_hover = hover_idx().is_some_and(|h| i == h);
                    let is_current = current_idx == Some(i);
                    rsx! {
                        button {
                            key: "{i}",
                            background: "none",
                            border: "none",
                            padding: "0.1rem 0.075rem",
                            cursor: "pointer",
                            font_size: "1.2rem",
                            line_height: "1",
                            color: if filled { "#f5c518" } else { "var(--surface0)" },
                            transform: if is_hover { "scale(1.35)" } else if filled { "scale(1.05)" } else { "scale(1)" },
                            transition: "color 0.1s, transform 0.12s",
                            display: "inline-block",
                            onmouseenter: move |_| hover_idx.set(Some(i)),
                            onmouseleave: move |_| hover_idx.set(None),
                            onclick: move |_| async move {
                                let new = if is_current { Rating::Neutral } else { this_rating };
                                let _ = set_rating(id, new).await;
                                if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                                    a.rating = if is_current { None } else { Some(new) };
                                }
                            },
                            Icon { icon: fa_solid_icons::FaStar, width: 15, height: 15 }
                        }
                    }
                }
            }
        }
    }
}
