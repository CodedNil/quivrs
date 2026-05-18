use crate::shared::{
    ArticleStatus, Rating,
    server_functions::{set_article_status, set_item_rating, set_rating},
};
use dioxus::prelude::*;
use uuid::Uuid;

#[allow(dead_code)]
pub mod base16 {
    pub const BASE00: &str = "#24273a"; // Default Background
    pub const BASE01: &str = "#1e2030"; // Lighter Background
    pub const BASE02: &str = "#363a4f"; // Selection Background
    pub const BASE03: &str = "#494d64"; // Comments, Invisibles, Line Highlighting
    pub const BASE04: &str = "#5b6078"; // Dark Foreground
    pub const BASE05: &str = "#cad3f5"; // Default Foreground
    pub const BASE06: &str = "#f4dbd6"; // Light Foreground
    pub const BASE07: &str = "#b7bdf8"; // Lightest Foreground
    pub const BASE08: &str = "#ed8796"; // Red: Variables, XML Tags, Markup Link Text, Markup Lists, Diff Deleted
    pub const BASE09: &str = "#f5a97f"; // Orange: Integers, Boolean, Constants, XML Attributes, Markup Link Url
    pub const BASE0A: &str = "#eed49f"; // Yellow: Classes, Markup Bold, Search Text Background
    pub const BASE0B: &str = "#a6da95"; // Green: Strings, Inherited Class, Markup Code, Diff Inserted
    pub const BASE0C: &str = "#8bd5ca"; // Cyan: Support, Regular Expressions, Escape Characters, Markup Quotes
    pub const BASE0D: &str = "#8aadf4"; // Blue: Functions, Methods, Attribute IDs, Headings
    pub const BASE0E: &str = "#c6a0f6"; // Magenta: Keywords, Storage, Selector, Markup Italic, Diff Changed
    pub const BASE0F: &str = "#f0c6c6"; // Brown: Deprecated, Opening/Closing Embedded Language Tags, e.g. <?php ?>
}

pub const fn rating_color(r: Rating) -> &'static str {
    match r {
        Rating::Hated => "#e06c75",
        Rating::Disliked => "#d4896a",
        Rating::Neutral => "#6c7680",
        Rating::Liked => "#74a679",
        Rating::Loved => "#98c379",
    }
}

pub const fn status_color(s: ArticleStatus) -> &'static str {
    match s {
        ArticleStatus::New => "#61afef",
        ArticleStatus::Stored => "#56b6c2",
        ArticleStatus::Binned => "#6c7680",
    }
}

#[component]
pub fn image_figure(url: String, caption: String, margin: String) -> Element {
    rsx! {
        figure { margin,
            img {
                src: "{url}",
                alt: "{caption}",
                width: "100%",
                border_radius: "0.375rem",
                display: "block",
            }
            figcaption {
                font_size: "0.7rem",
                color: base16::BASE03,
                margin_top: "0.375rem",
                text_align: "center",
                "{caption}"
            }
        }
    }
}

pub fn clean_url(url: &str) -> String {
    let s = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let s = s.strip_prefix("www.").unwrap_or(s);
    s.split('/').next().unwrap_or(s).to_string()
}

pub fn source_parts(s: &str) -> (&str, &str) {
    s.split_once('~').unwrap_or(("", s))
}

pub fn render_inline(text: &str) -> Element {
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
            if b == b'*' {
                elements.push(rsx! {
                    span { font_weight: "700", "{s}" }
                });
            } else {
                elements.push(rsx! {
                    span { font_style: "italic", "{s}" }
                });
            }
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

pub fn render_box_item(item: &str) -> Element {
    rsx! {
        div {
            background_color: base16::BASE01,
            border: "1px solid {base16::BASE02}",
            border_radius: "0.375rem",
            padding: "0.625rem 0.875rem",
            if let Some((header, text)) = item.split_once('~') {
                div {
                    font_size: "0.62rem",
                    font_weight: "700",
                    color: base16::BASE03,
                    text_transform: "uppercase",
                    letter_spacing: "0.08em",
                    margin_bottom: "0.25rem",
                    "{header}"
                }
                div {
                    font_size: "0.875rem",
                    color: base16::BASE05,
                    line_height: "1.5",
                    {render_inline(text)}
                }
            } else {
                div {
                    font_size: "0.875rem",
                    color: base16::BASE05,
                    line_height: "1.5",
                    {render_inline(item)}
                }
            }
        }
    }
}

pub fn item_rating_btn(
    label: &'static str,
    btn_rating: Rating,
    current_rating: Option<Rating>,
    item_key: String,
    mut ir_rev: Signal<u32>,
) -> Element {
    let active = current_rating == Some(btn_rating);
    let color = rating_color(btn_rating);
    let bg = if active { color } else { "transparent" };
    let text_col = if active {
        base16::BASE01
    } else {
        base16::BASE04
    };
    let border = format!("1px solid {}", if active { color } else { base16::BASE02 });
    rsx! {
        button {
            font_size: "0.6rem",
            padding: "0.15rem 0.5rem",
            border_radius: "9999px",
            background_color: bg,
            color: text_col,
            border,
            cursor: "pointer",
            onclick: move |_| {
                let k = item_key.clone();
                spawn(async move {
                    let _ = set_item_rating(k, btn_rating).await;
                    *ir_rev.write() += 1;
                });
            },
            "{label}"
        }
    }
}

#[component]
pub fn rated_tag(
    label: String,
    item_key: String,
    rating: Option<Rating>,
    tag_color: &'static str,
    ir_rev: Signal<u32>,
) -> Element {
    let mut hovered = use_signal(|| false);
    let dot_color = rating.map_or("transparent", rating_color);
    let border = format!("1px solid {tag_color}");

    rsx! {
        div {
            position: "relative",
            display: "inline-flex",
            align_items: "center",
            gap: "0.3rem",
            font_size: "0.62rem",
            padding: "0.125rem 0.625rem",
            background_color: "rgba(0,0,0,0.15)",
            border,
            border_radius: "9999px",
            color: tag_color,
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            div {
                width: "5px",
                height: "5px",
                min_width: "5px",
                border_radius: "50%",
                background_color: dot_color,
                opacity: if rating.is_some() { "1" } else { "0" },
            }
            span { "{label}" }
            if *hovered.read() {
                div {
                    position: "absolute",
                    top: "calc(100% + 4px)",
                    left: "0",
                    z_index: "10",
                    background_color: base16::BASE01,
                    border: "1px solid {base16::BASE02}",
                    border_radius: "0.5rem",
                    padding: "0.25rem 0.375rem",
                    display: "flex",
                    gap: "0.25rem",
                    {item_rating_btn("Hate", Rating::Hated, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Dislike", Rating::Disliked, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Neutral", Rating::Neutral, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Like", Rating::Liked, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Love", Rating::Loved, rating, item_key, ir_rev)}
                }
            }
        }
    }
}

#[component]
pub fn source_pill(url: String, rating: Option<Rating>, ir_rev: Signal<u32>) -> Element {
    let mut hovered = use_signal(|| false);
    let domain = clean_url(&url);
    let item_key = format!("source:{domain}");
    let dot_color = rating.map_or("transparent", rating_color);

    rsx! {
        div {
            position: "relative",
            display: "inline-flex",
            align_items: "center",
            gap: "0.3rem",
            font_size: "0.75rem",
            padding: "0.25rem 0.75rem",
            background_color: base16::BASE02,
            border: "1px solid {base16::BASE02}",
            border_radius: "9999px",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            div {
                width: "5px",
                height: "5px",
                min_width: "5px",
                border_radius: "50%",
                background_color: dot_color,
                opacity: if rating.is_some() { "1" } else { "0" },
            }
            a {
                href: "{url}",
                target: "_blank",
                rel: "noopener noreferrer",
                color: base16::BASE05,
                text_decoration: "none",
                "{domain}"
            }
            if *hovered.read() {
                div {
                    position: "absolute",
                    top: "calc(100% + 4px)",
                    left: "0",
                    z_index: "10",
                    background_color: base16::BASE01,
                    border: "1px solid {base16::BASE02}",
                    border_radius: "0.5rem",
                    padding: "0.25rem 0.375rem",
                    display: "flex",
                    gap: "0.25rem",
                    {item_rating_btn("Hate", Rating::Hated, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Dislike", Rating::Disliked, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Neutral", Rating::Neutral, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Like", Rating::Liked, rating, item_key.clone(), ir_rev)}
                    {item_rating_btn("Love", Rating::Loved, rating, item_key, ir_rev)}
                }
            }
        }
    }
}

pub fn status_button(
    label: &str,
    this_status: ArticleStatus,
    current: ArticleStatus,
    id: Uuid,
    mut ua_rev: Signal<u32>,
) -> Element {
    let active = current == this_status;
    let color = status_color(this_status);
    let bg = if active { color } else { "transparent" };
    let text_col = if active {
        base16::BASE01
    } else {
        base16::BASE04
    };
    let border = format!("1px solid {}", if active { color } else { base16::BASE02 });
    rsx! {
        button {
            font_size: "0.62rem",
            font_weight: if active { "600" } else { "400" },
            padding: "0.2rem 0.625rem",
            border_radius: "9999px",
            background_color: bg,
            color: text_col,
            border,
            cursor: "pointer",
            onclick: move |_| {
                spawn(async move {
                    let _ = set_article_status(id, this_status).await;
                    *ua_rev.write() += 1;
                });
            },
            "{label}"
        }
    }
}

pub fn rating_button(
    label: &str,
    this_rating: Rating,
    current: Option<Rating>,
    id: Uuid,
    mut ua_rev: Signal<u32>,
) -> Element {
    let active = current == Some(this_rating);
    let color = rating_color(this_rating);
    let bg = if active { color } else { "transparent" };
    let text_col = if active {
        base16::BASE01
    } else {
        base16::BASE04
    };
    let border = format!("1px solid {}", if active { color } else { base16::BASE02 });
    rsx! {
        button {
            font_size: "0.62rem",
            font_weight: if active { "600" } else { "400" },
            padding: "0.2rem 0.625rem",
            border_radius: "9999px",
            background_color: bg,
            color: text_col,
            border,
            cursor: "pointer",
            onclick: move |_| {
                spawn(async move {
                    let _ = set_rating(id, this_rating).await;
                    *ua_rev.write() += 1;
                });
            },
            "{label}"
        }
    }
}
