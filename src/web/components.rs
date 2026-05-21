use super::rating_color;
use crate::shared::{Rating, server_functions::set_item_rating};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape, icons::fa_solid_icons};
use std::collections::HashMap;

#[component]
pub fn RefreshButton(title: String, onclick: EventHandler<MouseEvent>) -> Element {
    let mut hovered = use_signal(|| false);
    rsx! {
        button {
            font_size: "0.875rem",
            line_height: "1",
            padding: "0.2rem 0.5rem",
            border_radius: "9999px",
            background_color: "var(--base02)",
            color: if hovered() { "var(--base06)" } else { "var(--base05)" },
            border: "none",
            cursor: "pointer",
            transition: "color 0.15s ease",
            title,
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            onclick,
            "↻"
        }
    }
}

/// A rating pill that stays fixed in place on hover and expands rating buttons
/// to either side via absolute positioning + clip-path animation.
#[component]
pub fn RatingPill(
    label: String,
    item_key: String,
    item_ratings: Signal<HashMap<String, Rating>>,
    url: Option<String>,
) -> Element {
    let current = item_ratings.read().get(&item_key).copied();
    let bg = rating_color(current.unwrap_or(Rating::Neutral));
    let mut hovered = use_signal(|| false);

    rsx! {
        div {
            position: "relative",
            display: "inline-flex",
            align_items: "center",
            z_index: if hovered() { "20" } else { "1" },
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),

            // Left buttons: grow leftward from behind the pill
            div {
                position: "absolute",
                right: "calc(100% - 0.5rem)",
                top: "0",
                bottom: "0",
                display: "flex",
                align_items: "center",
                background_color: bg,
                border_radius: "9999px 0 0 9999px",
                padding: "0.2rem 0.25rem 0.2rem 0.5rem",
                // Reveal from right (pill edge) outward to left
                clip_path: if hovered() { "inset(0 0% 0 0)" } else { "inset(0 0% 0 100%)" },
                transition: "clip-path 0.2s ease",
                RatingPillBtn {
                    icon: fa_solid_icons::FaAnglesLeft,
                    target: Rating::Hated,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
                RatingPillBtn {
                    icon: fa_solid_icons::FaAngleLeft,
                    target: Rating::Disliked,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
            }

            // Core pill — never moves, gains a dark outline ring on hover to mask neighbors
            div {
                display: "inline-flex",
                align_items: "center",
                background_color: bg,
                border_radius: "9999px",
                padding: "0.2rem 0.625rem",
                color: "var(--base05)",
                font_size: "0.62rem",
                font_weight: "700",
                position: "relative",
                z_index: "1",
                box_shadow: if hovered() { "0 0 0 3px var(--base00)" } else { "none" },
                transition: "box-shadow 0.12s ease",
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

            // Right buttons: grow rightward from behind the pill
            div {
                position: "absolute",
                left: "calc(100% - 0.5rem)",
                top: "0",
                bottom: "0",
                display: "flex",
                align_items: "center",
                background_color: bg,
                border_radius: "0 9999px 9999px 0",
                padding: "0.2rem 0.5rem 0.2rem 0.25rem",
                // Reveal from left (pill edge) outward to right
                clip_path: if hovered() { "inset(0 0 0 0%)" } else { "inset(0 0 0 100%)" },
                transition: "clip-path 0.2s ease",
                RatingPillBtn {
                    icon: fa_solid_icons::FaAngleRight,
                    target: Rating::Liked,
                    current,
                    item_key: item_key.clone(),
                    item_ratings,
                }
                RatingPillBtn {
                    icon: fa_solid_icons::FaAnglesRight,
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
pub fn RatingPillBtn<T: IconShape + Clone + PartialEq + 'static>(
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
