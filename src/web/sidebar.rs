use super::components::RefreshButton;
use super::{Route, rating_color, style};
use crate::shared::{
    ArticleData, ArticleStatus, Category, Rating, StoredArticle,
    server_functions::{get_user_articles, reclassify_articles},
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::fa_solid_icons};
use std::collections::BTreeMap;
use uuid::Uuid;

fn category_icon(category: Category, size: u32) -> Element {
    macro_rules! ico {
        ($i:expr) => {
            rsx! { Icon { width: size, height: size, fill: "currentColor", icon: $i } }
        };
    }
    match category {
        Category::Business => ico!(fa_solid_icons::FaBriefcase),
        Category::Politics => ico!(fa_solid_icons::FaLandmark),
        Category::Law => ico!(fa_solid_icons::FaGavel),
        Category::Health => ico!(fa_solid_icons::FaHeart),
        Category::Culture => ico!(fa_solid_icons::FaFilm),
        Category::Lifestyle => ico!(fa_solid_icons::FaHouse),
        Category::Transport => ico!(fa_solid_icons::FaCar),
        Category::Nature => ico!(fa_solid_icons::FaLeaf),
        Category::Technology => ico!(fa_solid_icons::FaMicrochip),
        Category::Software => ico!(fa_solid_icons::FaCode),
        Category::AI => ico!(fa_solid_icons::FaBrain),
        Category::Science => ico!(fa_solid_icons::FaFlask),
        Category::Sports => ico!(fa_solid_icons::FaFootball),
        Category::Gaming => ico!(fa_solid_icons::FaGamepad),
    }
}

#[component]
pub fn Sidebar(tab: String, selected_id: Option<Uuid>, active_status: ArticleStatus) -> Element {
    let mut articles: Signal<Vec<ArticleData>> = use_context();
    let data = articles.read().clone();

    let new_count = data
        .iter()
        .filter(|a| a.status == ArticleStatus::New)
        .count();
    let stored_count = data
        .iter()
        .filter(|a| a.status == ArticleStatus::Stored)
        .count();
    let binned_count = data
        .iter()
        .filter(|a| a.status == ArticleStatus::Binned)
        .count();
    let active: Vec<&ArticleData> = data.iter().filter(|a| a.status == active_status).collect();

    let mut groups: BTreeMap<Category, Vec<&ArticleData>> = BTreeMap::new();
    for a in &active {
        groups.entry(a.article.category).or_default().push(a);
    }
    let categories: Vec<Category> = groups.keys().copied().collect();

    let list = rsx! {
        for (category, items) in groups {
            CategoryGroup { category,
                for a in items {
                    ArticleItem {
                        key: "{a.id}",
                        id: a.id,
                        rating: a.rating,
                        article: a.article.clone(),
                        selected: selected_id,
                        tab: tab.clone(),
                    }
                }
            }
        }
    };

    let mut bar_fraction: Signal<f64> = use_signal(|| 0.0);
    let mut cat_heights: Signal<Vec<(Category, f64)>> = use_signal(Vec::new);
    #[cfg(target_arch = "wasm32")]
    let cats_for_scroll = categories.clone();
    #[cfg(target_arch = "wasm32")]
    let cats_for_effect = categories.clone();

    // Initialise category heights once the DOM is ready, so the minimap shows proportional sizes before the user scrolls.
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let cats = cats_for_effect.clone();
        spawn(async move {
            let _ = gloo_timers::future::TimeoutFuture::new(80).await;
            let doc = web_sys::window().and_then(|w| w.document());
            let heights: Vec<(Category, f64)> = cats
                .iter()
                .filter_map(|&cat| {
                    doc.as_ref()
                        .and_then(|d| d.get_element_by_id(&format!("category-group-{cat}")))
                        .map(|el| (cat, el.get_bounding_client_rect().height()))
                })
                .collect();
            if !heights.is_empty() {
                cat_heights.set(heights);
            }
        });
    });

    rsx! {
        div {
            width: "30rem",
            display: "flex",
            flex_direction: "column",
            background_color: "var(--base0d)",
            position: "relative",
            overflow: "hidden",

            div {
                id: "sidebar-header",
                display: "flex",
                flex_direction: "column",
                gap: "0.8rem",
                padding: "0.8rem",
                background_color: "var(--base01)",
                box_shadow: "0 4px 20px rgba(0,0,0,0.5)",

                div {
                    display: "flex",
                    justify_content: "space-between",
                    align_items: "center",
                    h1 {
                        font_size: "2rem",
                        font_weight: "800",
                        letter_spacing: "0.05em",
                        color: "var(--base05)",
                        text_transform: "uppercase",
                        margin: "0",
                        "Quivrs"
                    }
                    RefreshButton {
                        title: "Re-classify all articles",
                        onclick: move |_| async move {
                            let ids = articles.read().iter().map(|a| a.id).collect();
                            if reclassify_articles(ids).await.is_ok()
                                && let Ok(new_articles) = get_user_articles().await
                            {
                                articles.set(new_articles);
                            }
                        },
                    }
                }

                TabNav {
                    tab,
                    new_count,
                    stored_count,
                    binned_count,
                }
                CategoryScrollBar { categories, bar_fraction, cat_heights }
            }

            div {
                id: "article-scroll-container",
                flex: "1",
                overflow_y: "auto",
                onscroll: move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        let doc = web_sys::window().and_then(|w| w.document());
                        let scroll_el = doc
                            .as_ref()
                            .and_then(|d| d.get_element_by_id("article-scroll-container"));
                        if let Some(scroll_el) = scroll_el {
                            let scroll_top = scroll_el.scroll_top() as f64;
                            let max =
                                (scroll_el.scroll_height() - scroll_el.client_height()) as f64;
                            if max <= 0.0 {
                                return;
                            }
                            let sr = scroll_el.get_bounding_client_rect();

                            let segments: Vec<(Category, f64, f64)> = cats_for_scroll
                                .iter()
                                .filter_map(|&cat| {
                                    doc.as_ref()
                                        .and_then(|d| {
                                            d.get_element_by_id(&format!("category-group-{cat}"))
                                        })
                                        .map(|el| {
                                            let cr = el.get_bounding_client_rect();
                                            (cat, cr.top() - sr.top() + scroll_top, cr.height())
                                        })
                                })
                                .collect();
                            let total_h: f64 = segments.iter().map(|(_, _, h)| h).sum();
                            if total_h <= 0.0 {
                                return;
                            }
                            let new_heights: Vec<(Category, f64)> = segments
                                .iter()
                                .map(|(c, _, h)| (*c, *h))
                                .collect();
                            if *cat_heights.read() != new_heights {
                                cat_heights.set(new_heights);
                            }
                            let mut cum_bar = 0.0f64;
                            let mut found = false;
                            for &(_cat, cat_top, cat_h) in &segments {
                                let section_w = cat_h / total_h;
                                if cat_top <= scroll_top && scroll_top < cat_top + cat_h {
                                    let progress = if cat_h > 0.0 {
                                        (scroll_top - cat_top) / cat_h
                                    } else {
                                        0.0
                                    };
                                    bar_fraction
                                        .set((cum_bar + progress * section_w).clamp(0.0, 1.0));
                                    found = true;
                                    break;
                                }
                                if cat_top > scroll_top {
                                    bar_fraction.set(cum_bar);
                                    found = true;
                                    break;
                                }
                                cum_bar += section_w;
                            }
                            if !found {
                                bar_fraction.set(1.0);
                            }
                        }
                    }
                },
                {list}
            }
        }
    }
}

#[component]
fn TabNav(tab: String, new_count: usize, stored_count: usize, binned_count: usize) -> Element {
    let bubble_left = match tab.as_str() {
        "stored" => "0",
        "new" => "calc((100% - 0.5rem) / 3 + 0.25rem)",
        _ => "calc((100% - 0.5rem) * 2 / 3 + 0.5rem)",
    };

    rsx! {
        div { position: "relative", display: "flex", gap: "0.25rem",

            // Sliding bubble — sits behind button text, animates between tabs
            div {
                position: "absolute",
                top: "0",
                bottom: "0",
                left: "{bubble_left}",
                width: "calc((100% - 0.5rem) / 3)",
                background_color: "color-mix(in srgb, var(--base0d) 80%, var(--base01))",
                border_radius: "0.5rem",
                transition: "left 0.45s cubic-bezier(0.34, 1.56, 0.64, 1)",
                pointer_events: "none",
                z_index: "1",
            }

            TabButton {
                slug: "stored",
                label: "Stored",
                count: stored_count,
                active: tab.clone(),
            }
            TabButton {
                slug: "new",
                label: "New",
                count: new_count,
                active: tab.clone(),
            }
            TabButton {
                slug: "binned",
                label: "Binned",
                count: binned_count,
                active: tab,
            }
        }
    }
}

#[component]
fn TabButton(slug: &'static str, label: &'static str, count: usize, active: String) -> Element {
    let is_active = active.as_str() == slug;
    let nav = use_navigator();
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);
    let is_hovered = *hovered.read();
    let is_pressed = *pressed.read();

    let bg = if is_pressed {
        "rgba(255,255,255,0.14)"
    } else if is_hovered && !is_active {
        "rgba(255,255,255,0.11)"
    } else {
        "rgba(255,255,255,0.06)"
    };

    rsx! {
        button {
            flex: "1",
            position: "relative",
            z_index: "2",
            display: "flex",
            align_items: "center",
            justify_content: "center",
            gap: "0.375rem",
            padding: "0.5rem",
            border_radius: "0.5rem",
            border: "none",
            background_color: bg,
            cursor: "pointer",
            color: "var(--base05)",
            font_size: "0.875rem",
            font_weight: "700",
            transform: if is_pressed { "scale(0.94)" } else { "scale(1)" },
            transition: "background-color 0.15s ease, transform 0.1s ease",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| {
                hovered.set(false);
                pressed.set(false);
            },
            onmousedown: move |_| pressed.set(true),
            onmouseup: move |_| pressed.set(false),
            onclick: move |_| {
                nav.push(Route::TabHome {
                    tab: slug.to_string(),
                });
            },
            "{label}"
            span {
                background_color: "rgba(255,255,255,0.12)",
                color: "var(--base05)",
                border_radius: "9999px",
                padding: "0.125rem 0.4rem",
                font_size: "0.72rem",
                "{count}"
            }
        }
    }
}

#[component]
fn CategoryScrollBar(
    categories: Vec<Category>,
    bar_fraction: Signal<f64>,
    cat_heights: Signal<Vec<(Category, f64)>>,
) -> Element {
    let mut dragging = use_signal(|| false);

    if categories.is_empty() {
        return rsx! {};
    }

    let heights = cat_heights.read();
    let total_h: f64 = heights.iter().map(|(_, h)| h).sum();
    let category_flexes: Vec<(Category, f64)> = categories
        .iter()
        .map(|&cat| {
            let flex = if total_h > 0.0 {
                heights
                    .iter()
                    .find(|(c, _)| *c == cat)
                    .map_or(1.0, |(_, h)| *h)
            } else {
                1.0
            };
            (cat, flex)
        })
        .collect();
    drop(heights);

    let fraction = *bar_fraction.read();
    let frac_pct = fraction * 100.0;

    rsx! {
        div {
            id: "category-scroll-bar",
            position: "relative",
            display: "flex",
            align_items: "stretch",
            height: "1.7rem",
            gap: "0.2rem",
            onmousedown: move |_| dragging.set(true),
            onmouseup: move |_| dragging.set(false),
            onmouseleave: move |_| dragging.set(false),
            onmousemove: move |e| {
                #[cfg(target_arch = "wasm32")]
                if *dragging.read()
                    && let Some(bar) = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.get_element_by_id("category-scroll-bar"))
                {
                    let rect = bar.get_bounding_client_rect();
                    let frac = ((e.client_coordinates().x as f64 - rect.left()) / rect.width())
                        .clamp(0.0, 1.0);
                    if let Some(scroll_el) = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.get_element_by_id("article-scroll-container"))
                    {
                        let max = (scroll_el.scroll_height() - scroll_el.client_height()) as f64;
                        scroll_el.set_scroll_top((frac * max) as i32);
                    }
                }
            },

            // Position dot — behind the icon segments so it doesn't clutter
            // the label readability; the glow bleeds through the semi-transparent
            // segment backgrounds creating a lit-up effect
            div {
                position: "absolute",
                top: "50%",
                left: "clamp(0rem, calc({frac_pct:.2}% - 0.6rem), calc(100% - 1.2rem))",
                width: "1.2rem",
                height: "1.2rem",
                transform: "translateY(-50%)",
                background_color: "var(--base05)",
                border_radius: "9999px",
                pointer_events: "none",
                z_index: "1",
                transition: "left 0.06s ease",
            }

            for &(category, flex) in &category_flexes {
                div {
                    flex: "{flex:.1}",
                    position: "relative",
                    z_index: "2",
                    display: "flex",
                    align_items: "center",
                    justify_content: "flex-start",
                    padding_left: "0.375rem",
                    background_color: "rgba(255,255,255,0.08)",
                    border_radius: "0.3rem",
                    color: "rgba(255,255,255,0.6)",
                    cursor: "pointer",
                    title: "{category}",
                    onclick: move |e| {
                        e.stop_propagation();
                        #[cfg(target_arch = "wasm32")]
                        if let (Some(scroll_el), Some(cat_el)) = (
                            web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| d.get_element_by_id("article-scroll-container")),
                            web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| {
                                    d.get_element_by_id(&format!("category-group-{category}"))
                                }),
                        ) {
                            let sr = scroll_el.get_bounding_client_rect();
                            let cr = cat_el.get_bounding_client_rect();
                            let new_top = scroll_el.scroll_top() as f64 + cr.top() - sr.top()
                                - 8.0;
                            scroll_el.set_scroll_top(new_top.max(0.0) as i32);
                        }
                    },
                    {category_icon(category, 13)}
                }
            }
        }
    }
}

#[component]
fn CategoryGroup(category: Category, children: Element) -> Element {
    rsx! {
        div {
            id: "category-group-{category}",
            display: "flex",
            margin: "0 0.625rem 1.5rem",
            border_radius: style::RADIUS_GROUP,
            background_color: "rgba(54, 58, 79, 0.25)",
            overflow: "hidden",
            box_shadow: "inset 0 2px 6px rgba(0,0,0,0.18), 0 1px 4px rgba(0,0,0,0.22)",

            div { width: "44px", background_color: "rgba(30, 32, 48, 0.55)",
                div {
                    position: "sticky",
                    top: "0",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "flex-start",
                    padding: "0.875rem 0 0.875rem 0.5rem",
                    gap: "0.75rem",
                    color: "var(--base05)",
                    {category_icon(category, 24)}
                    div {
                        writing_mode: "vertical-rl",
                        transform: "rotate(180deg)",
                        font_size: "1.2rem",
                        font_weight: "800",
                        color: "var(--base05)",
                        text_transform: "uppercase",
                        letter_spacing: "0.12em",
                        "{category}"
                    }
                }
            }
            div { flex: "1", padding: "0.25rem 0", {children} }
        }
    }
}

#[component]
fn ArticleItem(
    id: Uuid,
    rating: Option<Rating>,
    article: StoredArticle,
    selected: Option<Uuid>,
    tab: String,
) -> Element {
    let is_selected = selected == Some(id);
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);
    let is_hovered = *hovered.read();
    let is_pressed = *pressed.read();

    let title = article.display_title();
    let description = {
        let full = article.display_description();
        match full.char_indices().nth(200) {
            Some((idx, _)) => &full[..idx],
            None => full,
        }
    };

    let hero_image = article.sources.iter().find_map(|s| {
        s.images
            .first()
            .and_then(|img| img.split('|').next())
            .filter(|url| !url.is_empty())
            .map(str::to_string)
    });

    let time_ago = {
        let diff = chrono::Utc::now().signed_duration_since(article.published);
        if diff.num_days() > 0 {
            format!("{}d", diff.num_days())
        } else if diff.num_hours() > 0 {
            format!("{}h", diff.num_hours())
        } else {
            format!("{}m", diff.num_minutes().max(1))
        }
    };

    let bg = if is_selected {
        "color-mix(in srgb, var(--base0d) 22%, var(--base00))"
    } else {
        "var(--base00)"
    };
    let scale = if is_pressed {
        "scale(0.98)"
    } else {
        "scale(1)"
    };
    let shadow = if is_hovered {
        "0 6px 16px rgba(0,0,0,0.8)"
    } else {
        "0 3px 8px rgba(0,0,0,0.6)"
    };
    let border_color = if is_hovered {
        "rgba(255,255,255,0.3)"
    } else {
        "rgba(255,255,255,0.18)"
    };

    rsx! {
        div {
            id: "article-{id}",
            cursor: "pointer",
            position: "relative",
            border_radius: style::RADIUS_CARD,
            margin: "0.75rem 0.875rem",
            overflow: "hidden",
            background_color: bg,
            border: "2px solid {border_color}",
            box_shadow: shadow,
            transform: scale,
            transition: "all 0.2s ease-out",
            aspect_ratio: "16 / 11",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| {
                hovered.set(false);
                pressed.set(false);
            },
            onmousedown: move |_| pressed.set(true),
            onmouseup: move |_| pressed.set(false),
            onclick: move |_| {
                use_navigator()
                    .push(Route::Article {
                        tab: tab.clone(),
                        id,
                    });
            },

            if let Some(img_url) = hero_image {
                img {
                    src: "{img_url}",
                    position: "absolute",
                    top: "0",
                    left: "0",
                    width: "100%",
                    height: "100%",
                    object_fit: "cover",
                    object_position: "center 35%",
                    z_index: "0",
                    transition: "transform 0.6s cubic-bezier(0.33, 1, 0.68, 1)",
                    transform: if is_hovered { "scale(1.08)" } else { "scale(1)" },
                }
            }

            if let Some(r) = rating {
                div {
                    position: "absolute",
                    top: "0",
                    left: "0",
                    width: "40px",
                    height: "40px",
                    z_index: 2,
                    filter: "drop-shadow(1px 2px 4px rgba(0,0,0,0.5))",
                    div {
                        width: "100%",
                        height: "100%",
                        background_color: rating_color(r),
                        clip_path: "polygon(0 0, 100% 0, 0 100%)",
                    }
                }
            }

            div {
                position: "absolute",
                bottom: "0",
                left: "0",
                right: "0",
                padding: "0.75rem 1rem 0.875rem 1rem",
                background: "linear-gradient(to top, rgba(0,0,0,0.8) 0%, rgba(0,0,0,0.5) 50%, transparent 100%)",
                backdrop_filter: "blur(16px)",
                z_index: "1",
                h3 {
                    font_size: "1rem",
                    font_weight: "700",
                    color: "#fff",
                    line_height: "1.2",
                    margin: "0 0 0.4rem 0",
                    text_shadow: "0 2px 10px rgba(0,0,0,0.9)",
                    "{title}"
                }
                p {
                    font_size: "0.75rem",
                    color: "rgba(255,255,255,0.85)",
                    line_height: "1.4",
                    margin: "0",
                    overflow: "hidden",
                    text_shadow: "0 1px 4px rgba(0,0,0,0.8)",
                    span {
                        font_size: "0.65rem",
                        color: "rgba(255,255,255,0.7)",
                        font_weight: "900",
                        text_transform: "uppercase",
                        letter_spacing: "0.05em",
                        margin_right: "0.6rem",
                        "{time_ago}"
                    }
                    "{description}"
                }
            }
        }
    }
}
