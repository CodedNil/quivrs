use super::components::{RatingPill, RefreshButton};
use crate::shared::{
    ArticleData, ArticleStatus, Rating, Section, StoredArticle,
    server_functions::{get_user_articles, reclassify_articles, set_article_status, set_rating},
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
    let hero = article.sources.iter().find_map(|s| {
        s.images
            .first()
            .and_then(|img| img.split('|').next())
            .filter(|u| !u.is_empty())
            .map(str::to_string)
    });

    rsx! {
        article {
            max_width: "44rem",
            margin_left: "auto",
            margin_right: "auto",
            padding_bottom: "5rem",

            // Hero image
            if let Some(url) = hero {
                img {
                    src: "{url}",
                    alt: "",
                    width: "100%",
                    max_height: "22rem",
                    object_fit: "cover",
                    display: "block",
                }
            }

            div { padding: "1.5rem 2rem 0",
                h1 {
                    font_size: "1.625rem",
                    font_weight: "800",
                    line_height: "1.2",
                    color: "var(--base05)",
                    margin: "0 0 0.75rem",
                    letter_spacing: "-0.01em",
                    "{title}"
                }

                // Meta: stored · date · rating · category · sources
                div {
                    display: "flex",
                    align_items: "center",
                    gap: "1.5rem",
                    margin_bottom: "1.25rem",
                    justify_content: "space-between",

                    div {
                        display: "flex",
                        align_items: "center",
                        gap: "0.375rem",
                        if status != ArticleStatus::Stored {
                            ActionBtn {
                                icon: fa_solid_icons::FaBookmark,
                                title: "Save to ReadL",
                                color: "var(--base0b)",
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
                                color: "var(--base08)",
                                onclick: move |_| async move {
                                    let _ = set_article_status(id, ArticleStatus::Binned).await;
                                    if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                                        a.status = ArticleStatus::Binned;
                                    }
                                },
                            }
                        }
                    }
                    span { font_size: "0.7rem", color: "var(--base04)",
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
                    RefreshButton {
                        title: "Re-classify article",
                        onclick: move |_| async move {
                            if reclassify_articles(vec![id]).await.is_ok()
                                && let Ok(updated) = get_user_articles().await
                            {
                                articles.set(updated);
                            }
                        },
                    }
                }

                // Description
                p {
                    font_size: "0.925rem",
                    color: "var(--base05)",
                    line_height: "1.65",
                    margin: "0 0 2rem",
                    font_style: "italic",
                    padding_left: "1rem",
                    border_left: "3px solid var(--base0d)",
                    opacity: "0.85",
                    "{description}"
                }

                // Content sections
                if let Some(entry) = &article.entry {
                    for section in &entry.sections {
                        {render_section(section)}
                    }
                } else {
                    p {
                        color: "var(--base04)",
                        font_size: "0.875rem",
                        font_style: "italic",
                        margin_bottom: "1rem",
                        "Generating summary…"
                    }
                    if let Some(s) = article.sources.first() {
                        div {
                            font_size: "0.825rem",
                            color: "var(--base04)",
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
            background_color: if hovered() { "var(--base02)" } else { "transparent" },
            color: if hovered() { color } else { "var(--base04)" },
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
    let current_idx = current.map(|r| match r {
        Rating::Hated => 0usize,
        Rating::Disliked => 1,
        Rating::Neutral => 2,
        Rating::Liked => 3,
        Rating::Loved => 4,
    });
    let mut hover_idx: Signal<Option<usize>> = use_signal(|| None);
    let fill_to = hover_idx.read().or(current_idx);

    rsx! {
        div { display: "flex", align_items: "center", gap: "0.05rem",
            for (i, &this_rating) in RATINGS.iter().enumerate() {
                {
                    let filled = fill_to.is_some_and(|h| i <= h);
                    let is_hover = hover_idx.read().is_some_and(|h| i == h);
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
                            color: if filled { "#f5c518" } else { "var(--base03)" },
                            transform: if is_hover { "scale(1.35)" } else if filled { "scale(1.05)" } else { "scale(1)" },
                            transition: "color 0.1s, transform 0.12s, text-shadow 0.15s",
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

fn render_inline(text: &str) -> Element {
    let mut elements: Vec<Element> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut plain_start = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if (b == b'*' || b == b'_')
            && let Some(rel_end) = bytes[i + 1..].iter().position(|&x| x == b)
        {
            let end = rel_end + i + 1;
            if i > plain_start {
                let s = text[plain_start..i].to_string();
                elements.push(rsx! {
                    span { "{s}" }
                });
            }
            let s = text[i + 1..end].to_string();
            elements.push(if b == b'*' {
                rsx! {
                    span { font_weight: "700", "{s}" }
                }
            } else {
                rsx! {
                    span { font_style: "italic", "{s}" }
                }
            });
            i = end + 1;
            plain_start = i;
            continue;
        }
        i += 1;
    }
    if plain_start < bytes.len() {
        let s = text[plain_start..].to_string();
        elements.push(rsx! {
            span { "{s}" }
        });
    }
    rsx! {
        {elements.into_iter()}
    }
}

fn render_box_item(item: &str) -> Element {
    let (header, text) = item
        .split_once('|')
        .map_or((None, item), |(h, t)| (Some(h), t));
    rsx! {
        div {
            background_color: "var(--base01)",
            border: "1px solid var(--base02)",
            border_radius: "0.375rem",
            padding: "0.625rem 0.875rem",
            height: "100%",
            if let Some(h) = header {
                div {
                    font_size: "0.62rem",
                    font_weight: "700",
                    color: "var(--base03)",
                    text_transform: "uppercase",
                    letter_spacing: "0.08em",
                    margin_bottom: "0.25rem",
                    "{h}"
                }
            }
            div {
                font_size: "0.875rem",
                color: "var(--base05)",
                line_height: "1.5",
                {render_inline(text)}
            }
        }
    }
}

fn render_section(section: &Section) -> Element {
    match section {
        Section::Header(s) => rsx! {
            h2 {
                font_size: "0.8rem",
                font_weight: "700",
                color: "var(--base05)",
                text_transform: "uppercase",
                letter_spacing: "0.1em",
                margin: "2.5rem 0 0.8rem",
                "{s}"
            }
        },
        Section::Paragraph(text) => rsx! {
            p {
                font_size: "0.875rem",
                color: "var(--base05)",
                line_height: "1.75",
                margin: "0 0 1rem",
                {render_inline(text)}
            }
        },
        Section::Image(raw) => {
            let (url, caption) = raw.split_once('|').unwrap_or(("", raw));
            rsx! {
                figure { margin: "1.25rem 0",
                    img {
                        src: "{url}",
                        alt: "{caption}",
                        width: "100%",
                        border_radius: "0.375rem",
                        display: "block",
                    }
                    figcaption {
                        font_size: "0.7rem",
                        color: "var(--base03)",
                        margin_top: "0.375rem",
                        text_align: "center",
                        "{caption}"
                    }
                }
            }
        }
        Section::List(items) => rsx! {
            ul {
                margin: "0 0 1.25rem",
                padding_left: "1.25rem",
                display: "flex",
                flex_direction: "column",
                gap: "0.375rem",
                for item in items {
                    li {
                        font_size: "0.875rem",
                        color: "var(--base05)",
                        line_height: "1.6",
                        if let Some((h, t)) = item.split_once('|') {
                            span { font_weight: "700", "{h}" }
                            " "
                            {render_inline(t)}
                        } else {
                            {render_inline(item)}
                        }
                    }
                }
            }
        },
        Section::RowBoxes(items) => rsx! {
            div {
                display: "flex",
                flex_direction: "column",
                gap: "0.5rem",
                margin: "0 0 1.25rem",
                for item in items {
                    {render_box_item(item)}
                }
            }
        },
        Section::ColumnBoxes(items) => rsx! {
            div {
                display: "flex",
                flex_wrap: "wrap",
                gap: "0.5rem",
                margin: "0 0 1.25rem",
                for item in items {
                    div { flex: "1", min_width: "0", {render_box_item(item)} }
                }
            }
        },
    }
}
