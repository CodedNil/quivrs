use crate::shared::{Rating, server_functions::set_item_rating};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape, icons::fa_solid_icons};
use std::collections::HashMap;

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

/// A rating pill that expands with rating buttons on hover.
#[component]
pub fn RatingPill(
    label: String,
    item_key: String,
    item_ratings: Signal<HashMap<String, Rating>>,
    url: Option<String>,
) -> Element {
    let current = item_ratings().get(&item_key).copied();
    let bg = current.unwrap_or(Rating::Neutral).color();
    let mut hovered = use_signal(|| false);

    rsx! {
        div {
            position: "relative",
            display: "inline-flex",
            align_items: "center",
            z_index: if hovered() { "20" } else { "1" },
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),

            // Expanding background — grows to cover button panels, carries the border
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

            // Left buttons — right edge flush with pill left edge
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

            // Core pill label — in-flow, sets the wrapper's size
            div {
                display: "inline-flex",
                align_items: "center",
                background_color: bg,
                border_radius: "9999px",
                padding: "0.2rem 0.625rem",
                color: "var(--text)",
                font_size: "0.62rem",
                font_weight: "700",
                white_space: "nowrap",
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

            // Right buttons — left edge flush with pill right edge
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
