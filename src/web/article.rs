use super::components::RatingPill;
use crate::shared::{
    ArticleData, ArticleStatus, Rating, StoredArticle,
    server_functions::{set_article_status, set_rating},
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
    let description = article.display_description();
    let hero = article.hero_image();

    rsx! {
        if let Some(url) = hero {
            img {
                src: "{url}",
                alt: "",
                width: "100%",
                max_height: "22rem",
                object_fit: "cover",
                display: "block",
                border_radius: "20px 20px 0 0",
            }
        }

        div {
            max_width: "44rem",
            margin_left: "auto",
            margin_right: "auto",
            padding: "1.5rem 2rem 5rem",

            h1 {
                font_size: "1.625rem",
                font_weight: "800",
                line_height: "1.2",
                color: "var(--text)",
                margin: "0 0 0.75rem",
                letter_spacing: "-0.01em",
                "{title}"
            }

            div {
                display: "flex",
                align_items: "center",
                gap: "1.5rem",
                margin_bottom: "1.25rem",
                justify_content: "space-between",

                div { display: "flex", align_items: "center", gap: "0.375rem",
                    if status != ArticleStatus::Stored {
                        ActionBtn {
                            icon: fa_solid_icons::FaBookmark,
                            title: "Save to ReadL",
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
                div { display: "flex", align_items: "center", gap: "0.375rem",
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

            p {
                font_size: "0.925rem",
                color: "var(--text)",
                line_height: "1.65",
                margin: "0 0 2rem",
                font_style: "italic",
                padding_left: "1rem",
                border_left: "3px solid var(--accent)",
                opacity: "0.85",
                "{description}"
            }

            if let Some(entry) = &article.entry {
                style {
                    "{ARTICLE_CSS}"
                }
                div {
                    class: "article-content",
                    dangerous_inner_html: "{entry.content}"
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

const ARTICLE_CSS: &str = r"
    .article-content {
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
        color: var(--text);
        line-height: 1.75;
        font-size: 0.9375rem;
    }
    .article-content h1 { font-size: 1.5rem; font-weight: 800; margin: 2rem 0 0.5rem; color: var(--text); }
    .article-content h2 { font-size: 1.25rem; font-weight: 700; margin: 1.5rem 0 0.5rem; color: var(--text); }
    .article-content h3 { font-size: 1.1rem; font-weight: 700; margin: 1rem 0 0.5rem; color: var(--text); }
    .article-content p { margin: 0; }
    .article-content img { width: 100%; border-radius: 12px; display: block; margin: 1rem 0; }
    .article-content ul { margin: 0; padding-left: 1.5rem; display: flex; flex-direction: column; gap: 0.5rem; }

    .article-content .box {
        border: 1px solid var(--surface0);
        border-radius: 10px;
        padding: 1rem;
        background: var(--mantle);
    }
    .article-content .flexbox-columns {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    .article-content .flexbox-rows {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
    }
    .article-content .flexbox-rows .box {
        flex: 1;
        min-width: 200px;
    }
    .article-content .quote {
        font-style: italic;
        padding-left: 1.25rem;
        border-left: 4px solid var(--accent);
        color: var(--subtext1);
        margin: 1rem 0;
    }
    .article-content figure { margin: 1.25rem 0; }
    .article-content figcaption {
        font_size: 0.75rem;
        color: var(--subtext1);
        margin-top: 0.5rem;
        text-align: center;
        line-height: 1.4;
    }
";

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
