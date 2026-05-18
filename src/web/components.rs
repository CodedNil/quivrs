use crate::shared::{
    ArticleData, ArticleStatus, Rating, Section,
    server_functions::{set_article_status, set_item_rating, set_rating},
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon, IconShape,
    icons::fa_solid_icons::{FaAngleLeft, FaAngleRight, FaAnglesLeft, FaAnglesRight},
};
use std::collections::HashMap;
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

pub fn clean_url(url: &str) -> String {
    let s = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let s = s.strip_prefix("www.").unwrap_or(s);
    s.split('/').next().unwrap_or(s).to_string()
}

fn source_parts(s: &str) -> (&str, &str) {
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

fn render_box_item(item: &str) -> Element {
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

pub fn render_section(section: &Section) -> Element {
    match section {
        Section::Header(header) => rsx! {
            h2 {
                font_size: "0.62rem",
                font_weight: "700",
                color: base16::BASE03,
                text_transform: "uppercase",
                letter_spacing: "0.1em",
                margin: "1.75rem 0 0.375rem 0",
                "{header}"
            }
        },
        Section::Paragraph(text) => rsx! {
            p {
                font_size: "0.875rem",
                color: base16::BASE05,
                line_height: "1.75",
                margin: "0 0 1rem 0",
                {render_inline(text)}
            }
        },
        Section::Image(raw) => {
            let (url, caption) = source_parts(raw);
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
                        color: base16::BASE03,
                        margin_top: "0.375rem",
                        text_align: "center",
                        "{caption}"
                    }
                }
            }
        }
        Section::List(items) => rsx! {
            ul {
                margin: "0 0 1.25rem 0",
                padding_left: "1.25rem",
                display: "flex",
                flex_direction: "column",
                gap: "0.375rem",
                for item in items {
                    li {
                        font_size: "0.875rem",
                        color: base16::BASE05,
                        line_height: "1.6",
                        if let Some((header, text)) = item.split_once('~') {
                            span { font_weight: "700", "{header}" }
                            " "
                            {render_inline(text)}
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
                margin: "0 0 1.25rem 0",
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
                margin: "0 0 1.25rem 0",
                for item in items {
                    div { flex: "1", min_width: "0", {render_box_item(item)} }
                }
            }
        },
    }
}

// Shared pill-shaped button used for all status/rating controls.
#[component]
fn PillButton(
    label: String,
    active: bool,
    color: &'static str,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let border_color = if active { color } else { base16::BASE02 };
    rsx! {
        button {
            font_size: "0.62rem",
            font_weight: if active { "600" } else { "400" },
            padding: "0.2rem 0.625rem",
            border_radius: "9999px",
            background_color: if active { color } else { "transparent" },
            color: if active { base16::BASE01 } else { base16::BASE04 },
            border: "1px solid {border_color}",
            cursor: "pointer",
            onclick,
            "{label}"
        }
    }
}

// A pill that shows a label and inline rating buttons
#[component]
pub fn RatingPill(
    label: String,
    item_key: String,
    item_ratings: Signal<HashMap<String, Rating>>,
    url: Option<String>,
) -> Element {
    let current = item_ratings.read().get(&item_key).copied();
    let bg_color = current.map_or(base16::BASE02, rating_color);
    let mut is_hovered = use_signal(|| false);

    rsx! {
        div {
            display: "inline-flex",
            align_items: "center",
            background_color: bg_color,
            border_radius: "9999px",
            overflow: "hidden",
            color: base16::BASE01,
            font_size: "0.62rem",
            font_weight: "700",
            transition: "all 0.2s ease-in-out",
            onmouseenter: move |_| is_hovered.set(true),
            onmouseleave: move |_| is_hovered.set(false),

            div {
                display: "flex",
                align_items: "center",
                max_width: if is_hovered() { "100px" } else { "0" },
                opacity: if is_hovered() { "1" } else { "0" },
                transition: "all 0.2s ease-in-out",
                overflow: "hidden",
                RatingPillBtn {
                    icon: FaAnglesLeft,
                    target: Rating::Hated,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
                RatingPillBtn {
                    icon: FaAngleLeft,
                    target: Rating::Disliked,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
            }

            div {
                padding: "0.125rem 0.625rem",
                background_color: "rgba(255, 255, 255, 0.1)",
                if let Some(href) = url {
                    a {
                        href: "{href}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        color: "inherit",
                        text_decoration: "none",
                        "{label}"
                    }
                } else {
                    span { "{label}" }
                }
            }

            div {
                display: "flex",
                align_items: "center",
                max_width: if is_hovered() { "100px" } else { "0" },
                opacity: if is_hovered() { "1" } else { "0" },
                transition: "all 0.2s ease-in-out",
                overflow: "hidden",
                RatingPillBtn {
                    icon: FaAngleRight,
                    target: Rating::Liked,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
                RatingPillBtn {
                    icon: FaAnglesRight,
                    target: Rating::Loved,
                    current,
                    item_key,
                    item_ratings,
                }
            }
        }
    }
}

#[component]
fn RatingPillBtn<T: IconShape + Clone + PartialEq + 'static>(
    icon: T,
    target: Rating,
    current: Option<Rating>,
    item_key: String,
    mut item_ratings: Signal<HashMap<String, Rating>>,
) -> Element {
    rsx! {
        button {
            padding: "0.125rem 0.375rem",
            background_color: "transparent",
            border: "none",
            cursor: "pointer",
            color: "inherit",
            display: "flex",
            align_items: "center",
            justify_content: "center",
            onclick: move |_| {
                let k = item_key.clone();
                async move {
                    let new_rating = if current == Some(target) {
                        Rating::Neutral
                    } else {
                        target
                    };
                    let _ = set_item_rating(k.clone(), new_rating).await;
                    item_ratings.write().insert(k, new_rating);
                }
            },
            Icon { icon, width: 12, height: 12 }
        }
    }
}

pub fn status_button(
    label: &str,
    this_status: ArticleStatus,
    current: ArticleStatus,
    id: Uuid,
    mut articles: Signal<Vec<ArticleData>>,
) -> Element {
    rsx! {
        PillButton {
            label: label.to_string(),
            active: current == this_status,
            color: status_color(this_status),
            onclick: move |_| async move {
                let _ = set_article_status(id, this_status).await;
                if let Some((_, s, _, _)) = articles
                    .write()
                    .iter_mut()
                    .find(|(i, _, _, _)| *i == id)
                {
                    *s = this_status;
                }
            },
        }
    }
}

pub fn rating_button(
    label: &str,
    this_rating: Rating,
    current: Option<Rating>,
    id: Uuid,
    mut articles: Signal<Vec<ArticleData>>,
) -> Element {
    rsx! {
        PillButton {
            label: label.to_string(),
            active: current == Some(this_rating),
            color: rating_color(this_rating),
            onclick: move |_| async move {
                let _ = set_rating(id, this_rating).await;
                if let Some((_, _, r, _)) = articles
                    .write()
                    .iter_mut()
                    .find(|(i, _, _, _)| *i == id)
                {
                    *r = Some(this_rating);
                }
            },
        }
    }
}
