use crate::shared::{StoredArticle, server_functions::get_articles};
use dioxus::prelude::*;

pub fn app() -> Element {
    let articles = use_resource(get_articles);
    let selected = use_signal(|| Option::<StoredArticle>::None);

    rsx! {
        document::Title { "Quivrs" }
        div {
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            background_color: "#09090b",
            color: "#d4d4d8",
            aside {
                width: "18rem",
                border_right: "1px solid #27272a",
                display: "flex",
                flex_direction: "column",
                background_color: "#18181b",
                div {
                    padding: "1rem 1.25rem",
                    border_bottom: "1px solid #27272a",
                    flex_shrink: "0",
                    h1 {
                        font_size: "0.75rem",
                        font_weight: "700",
                        letter_spacing: "0.1em",
                        color: "#ffffff",
                        text_transform: "uppercase",
                        margin: "0",
                        "Quivrs"
                    }
                }
                div { overflow_y: "auto", flex: "1",
                    match &*articles.read_unchecked() {
                        Some(Ok(list)) => rsx! {
                            for article in list {
                                article_item { key: "{article.id}", article: article.clone(), selected }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            p { padding: "1.25rem", font_size: "0.75rem", color: "#f87171", "Error: {e}" }
                        },
                        None => rsx! {
                            p { padding: "1.25rem", font_size: "0.75rem", color: "#52525b", "Loading..." }
                        },
                    }
                }
            }
            main { flex: "1", overflow_y: "auto",
                match selected.read().as_ref() {
                    Some(article) => rsx! {
                        article_detail { article: article.clone() }
                    },
                    None => rsx! {
                        div {
                            display: "flex",
                            align_items: "center",
                            justify_content: "center",
                            height: "100%",
                            color: "#52525b",
                            font_size: "0.875rem",
                            "Select an article to read"
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn article_item(article: StoredArticle, mut selected: Signal<Option<StoredArticle>>) -> Element {
    let is_selected = selected.read().as_ref().map(|a| a.id) == Some(article.id);
    let mut hovered = use_signal(|| false);

    let title = if article.entry.title.is_empty() {
        article
            .sources
            .first()
            .map_or_else(|| "Untitled".to_string(), |s| s.title.clone())
    } else {
        article.entry.title.clone()
    };
    let description = if article.entry.description.is_empty() {
        article
            .sources
            .first()
            .map_or_else(String::new, |s| s.description.clone())
    } else {
        article.entry.description.clone()
    };

    let bg = if is_selected {
        "rgba(23, 37, 84, 0.4)"
    } else if *hovered.read() {
        "rgba(39, 39, 42, 0.4)"
    } else {
        "transparent"
    };
    let bl = if is_selected {
        "3px solid #3b82f6"
    } else {
        "3px solid transparent"
    };
    let pl = if is_selected { "17px" } else { "1.25rem" };

    rsx! {
        div {
            padding_top: "0.875rem",
            padding_bottom: "0.875rem",
            padding_right: "1.25rem",
            padding_left: pl,
            cursor: "pointer",
            border_bottom: "1px solid rgba(39, 39, 42, 0.6)",
            border_left: bl,
            background_color: bg,
            onclick: move |_| selected.set(Some(article.clone())),
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            h3 {
                font_size: "0.75rem",
                font_weight: "600",
                color: "#e4e4e7",
                line_height: "1.375",
                margin: "0 0 0.25rem 0",
                "{title}"
            }
            if !description.is_empty() {
                p {
                    font_size: "0.68rem",
                    color: "#71717a",
                    line_height: "1.625",
                    margin: "0",
                    "{description}"
                }
            }
        }
    }
}

#[component]
fn article_detail(article: StoredArticle) -> Element {
    let title = if article.entry.title.is_empty() {
        article
            .sources
            .first()
            .map_or_else(|| "Untitled".to_string(), |s| s.title.clone())
    } else {
        article.entry.title.clone()
    };
    let updated = article.updated_at.format("%Y-%m-%d %H:%M UTC").to_string();

    rsx! {
        article {
            max_width: "42rem",
            margin_left: "auto",
            margin_right: "auto",
            padding_left: "2.5rem",
            padding_right: "2.5rem",
            padding_top: "3rem",
            padding_bottom: "5rem",
            h1 {
                font_size: "1.5rem",
                font_weight: "700",
                line_height: "1.25",
                color: "#f4f4f5",
                margin: "0 0 0.5rem 0",
                "{title}"
            }
            p {
                font_size: "0.7rem",
                color: "#52525b",
                margin: "0 0 1.25rem 0",
                "{updated}"
            }
            if !article.entry.description.is_empty() {
                p {
                    font_size: "0.875rem",
                    color: "#a1a1aa",
                    line_height: "1.625",
                    margin: "0 0 1.5rem 0",
                    font_style: "italic",
                    padding_left: "0.875rem",
                    border_left: "2px solid #3f3f46",
                    "{article.entry.description}"
                }
            }
            if !article.entry.tags.is_empty() {
                div {
                    display: "flex",
                    flex_wrap: "wrap",
                    gap: "0.375rem",
                    margin_bottom: "1.75rem",
                    for tag in &article.entry.tags {
                        span {
                            font_size: "0.62rem",
                            padding: "0.125rem 0.625rem",
                            background_color: "#27272a",
                            border: "1px solid #3f3f46",
                            border_radius: "9999px",
                            color: "#71717a",
                            "{tag}"
                        }
                    }
                }
            }
            if !article.entry.full_content.is_empty() {
                div {
                    font_size: "0.875rem",
                    color: "#d4d4d8",
                    dangerous_inner_html: "{article.entry.full_content}",
                }
            }
            if !article.sources.is_empty() {
                div {
                    margin_top: "2.25rem",
                    padding_top: "1.25rem",
                    border_top: "1px solid #27272a",
                    h4 {
                        font_size: "0.62rem",
                        color: "#52525b",
                        text_transform: "uppercase",
                        letter_spacing: "0.1em",
                        margin: "0 0 0.625rem 0",
                        "Sources"
                    }
                    for source in &article.sources {
                        a {
                            display: "block",
                            font_size: "0.75rem",
                            color: "#60a5fa",
                            margin_bottom: "0.375rem",
                            text_overflow: "ellipsis",
                            overflow: "hidden",
                            white_space: "nowrap",
                            href: "{source.url}",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            "{source.title}"
                        }
                    }
                }
            }
        }
    }
}
