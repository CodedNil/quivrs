use crate::shared::{Rating, server_functions::set_item_rating};
use dioxus::prelude::*;
use std::collections::HashMap;

macro_rules! material_icon_svg {
    ($name:literal) => {
        include_str!(concat!("../../assets/icons/", $name, ".svg"))
    };
}

#[component]
pub fn MaterialIcon(name: String, size: u32) -> Element {
    let size = size.saturating_mul(5) / 4;
    let svg = match name.as_str() {
        "keyboard_double_arrow_left" => material_icon_svg!("keyboard_double_arrow_left"),
        "keyboard_arrow_left" => material_icon_svg!("keyboard_arrow_left"),
        "keyboard_arrow_right" => material_icon_svg!("keyboard_arrow_right"),
        "keyboard_double_arrow_right" => material_icon_svg!("keyboard_double_arrow_right"),

        "star" => material_icon_svg!("star"),
        "delete" => material_icon_svg!("delete"),
        "bookmark_star" => material_icon_svg!("bookmark_star"),

        "Business" => material_icon_svg!("business_center"),
        "Politics" => material_icon_svg!("account_balance"),
        "Law" => material_icon_svg!("gavel"),
        "Health" => material_icon_svg!("health_metrics"),
        "Culture" => material_icon_svg!("theater_comedy"),
        "Lifestyle" => material_icon_svg!("chair"),
        "Transport" => material_icon_svg!("directions_car"),
        "Nature" => material_icon_svg!("nature"),
        "Technology" => material_icon_svg!("memory"),
        "Software" => material_icon_svg!("code"),
        "AI" => material_icon_svg!("neurology"),
        "Science" => material_icon_svg!("science"),
        "Sports" => material_icon_svg!("sports_and_outdoors"),
        "Gaming" => material_icon_svg!("sports_esports"),

        other => panic!("unsupported material icon: {other}"),
    };
    let svg = svg.replacen(
        "<svg ",
        "<svg style=\"fill:currentColor;width:100%;height:100%;display:block;\" ",
        1,
    );

    rsx! {
        span {
            class: "material-icon",
            style: "width: {size}px; height: {size}px;",
            aria_hidden: "true",
            dangerous_inner_html: "{svg}",
        }
    }
}

#[component]
pub fn CenteredMessage(text: String) -> Element {
    rsx! {
        div {
            display: "flex",
            align_items: "center",
            justify_content: "center",
            min_height: "100%",
            color: "var(--subtext0)",
            font_size: "0.875rem",
            "{text}"
        }
    }
}

/// A compact, non-interactive article attribute badge.
#[component]
pub fn InfoPill(label: String) -> Element {
    rsx! {
        span {
            display: "inline-flex",
            align_items: "center",
            background_color: "var(--surface0)",
            border_radius: "9999px",
            padding: "0.2rem 0.625rem",
            color: "var(--subtext1)",
            font_size: "0.62rem",
            font_weight: "700",
            white_space: "nowrap",
            "{label}"
        }
    }
}

/// A rating pill that expands with rating buttons on hover.
#[component]
pub fn RatingPill(label: String, item_key: String, urls: Vec<String>) -> Element {
    let item_ratings = use_context::<Signal<HashMap<String, Rating>>>();
    let current = item_ratings.read().get(&item_key).copied();
    let bg = current.unwrap_or(Rating::Neutral).color();
    let mut hovered = use_signal(|| false);
    let mut hover_idx = use_signal(|| None::<usize>);

    rsx! {
        div {
            position: "relative",
            display: "inline-flex",
            align_items: "center",
            z_index: if hovered() { "20" } else { "1" },
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),

            // Expanding background
            div {
                position: "absolute",
                left: if hovered() { "-3rem" } else { "0" },
                right: if hovered() { "-3rem" } else { "0" },
                top: "0",
                bottom: "0",
                background_color: bg,
                border_radius: "9999px",
                box_shadow: if hovered() { "0 0 3px 3px rgba(0,0,0,0.7)" } else { "none" },
                transition: "left 0.2s ease, right 0.2s ease, box-shadow 0.15s ease",
                z_index: "-1",
            }

            // Left rating buttons
            div {
                position: "absolute",
                right: "100%",
                top: "0",
                bottom: "0",
                display: "flex",
                align_items: "center",
                opacity: if hovered() { "1" } else { "0" },
                pointer_events: if hovered() { "auto" } else { "none" },
                transition: "opacity 0.15s ease 0.05s",
                RatingPillBtn {
                    icon: "keyboard_double_arrow_left",
                    target: Rating::Hated,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
                RatingPillBtn {
                    icon: "keyboard_arrow_left",
                    target: Rating::Disliked,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
            }

            // Core pill label — in-flow, sets the wrapper's size
            div {
                position: "relative",
                display: "inline-flex",
                align_items: "center",
                background_color: bg,
                border_radius: "9999px",
                padding: "0.2rem 0.625rem",
                color: "var(--text)",
                font_size: "0.62rem",
                font_weight: "700",
                white_space: "nowrap",

                // The label text
                if urls.len() > 1 {
                    span {
                        position: "relative",
                        z_index: "0",
                        pointer_events: "none",
                        "{label}"
                    }
                } else if urls.is_empty() {
                    span { "{label}" }
                } else {
                    a {
                        href: "{urls[0]}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        color: "inherit",
                        text_decoration: "none",
                        "{label}"
                    }
                }

                // Overlay link buttons for multi-URL domains
                if urls.len() > 1 {
                    div {
                        position: "absolute",
                        left: "0",
                        top: "0",
                        right: "0",
                        bottom: "0",
                        display: "flex",
                        gap: "2px",
                        padding: "2px",
                        z_index: "1",
                        for (i, url) in urls.iter().enumerate() {
                            a {
                                href: "{url}",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                flex: "1",
                                border_radius: "9999px",
                                background_color: if hover_idx() == Some(i) { "rgba(255,255,255,0.25)" } else { "transparent" },
                                transition: "background-color 0.15s ease",
                                display: "block",
                                aria_label: "{label} article {i + 1}",
                                onmouseenter: move |_| hover_idx.set(Some(i)),
                                onmouseleave: move |_| hover_idx.set(None),
                            }
                        }
                    }
                }
            }

            // Right rating buttons
            div {
                position: "absolute",
                left: "100%",
                top: "0",
                bottom: "0",
                display: "flex",
                align_items: "center",
                opacity: if hovered() { "1" } else { "0" },
                pointer_events: if hovered() { "auto" } else { "none" },
                transition: "opacity 0.15s ease 0.05s",
                RatingPillBtn {
                    icon: "keyboard_arrow_right",
                    target: Rating::Liked,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
                RatingPillBtn {
                    icon: "keyboard_double_arrow_right",
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
pub fn RatingPillBtn(
    icon: &'static str,
    target: Rating,
    current: Option<Rating>,
    item_key: String,
    mut item_ratings: Signal<HashMap<String, Rating>>,
) -> Element {
    rsx! {
        button {
            aria_label: "Set item rating",
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
            MaterialIcon { name: icon, size: 12 }
        }
    }
}
