use super::{Route, components::RefreshButton, rating_color};
use crate::shared::{
    ArticleData, ArticleStatus, Category, Rating, StoredArticle,
    server_functions::{get_user_articles, reclassify_articles},
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::fa_solid_icons};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Fixed width of the sidebar panel.
const SIDEBAR_WIDTH: &str = "30rem";

/// Width of the category label column.
const LABEL_WIDTH: &str = "44px";

/// Height of each article card.
const ARTICLE_HEIGHT_PX: u32 = 300;

/// Uniform gap between cards and inset from the category column edges.
const ARTICLE_GAP_PX: u32 = 12;

/// Separator border thickness below each category group.
const CATEGORY_BORDER_PX: u32 = 12;

#[cfg(target_arch = "wasm32")]
fn get_element(id: &str) -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(id)
}

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

    let cat_heights: Vec<(Category, f64)> = groups
        .iter()
        .map(|(&cat, items)| {
            let h = (items.len() as f64).mul_add(
                f64::from(ARTICLE_HEIGHT_PX + ARTICLE_GAP_PX),
                f64::from(ARTICLE_GAP_PX + CATEGORY_BORDER_PX),
            );
            (cat, h)
        })
        .collect();

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

    let mut scroll_top_val: Signal<f64> = use_signal(|| 0.0);

    rsx! {
        div {
            width: SIDEBAR_WIDTH,
            display: "flex",
            flex_direction: "column",
            background_color: "var(--accent)",
            position: "relative",
            overflow: "hidden",

            div {
                id: "sidebar-header",
                display: "flex",
                flex_direction: "column",
                gap: "0.8rem",
                padding: "0.8rem",
                background_color: "var(--mantle)",

                div {
                    display: "flex",
                    justify_content: "space-between",
                    align_items: "center",
                    h1 {
                        font_size: "2rem",
                        font_weight: "800",
                        letter_spacing: "0.05em",
                        color: "var(--text)",
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
                CategoryScrollBar {
                    categories,
                    scroll_top: scroll_top_val,
                    cat_heights,
                }
            }

            div {
                id: "article-scroll-container",
                flex: "1",
                overflow_y: "auto",
                onscroll: move |_| {
                    #[cfg(target_arch = "wasm32")]
                    if let Some(el) = get_element("article-scroll-container") {
                        scroll_top_val.set(el.scroll_top() as f64);
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
                background_color: "var(--accent))",
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
            color: "var(--text)",
            font_size: "0.875rem",
            font_weight: "700",
            transform: if is_pressed { "scale(0.94)" } else { "scale(1)" },
            transition: "background-color 0.15s ease, transform 0.1s ease",
            filter: "drop-shadow(0.5px 0.5px 1px rgba(0,0,0,0.6))",
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
                color: "var(--text)",
                border_radius: "9999px",
                padding: "0.125rem 0.4rem",
                font_size: "0.72rem",
                "{count}"
            }
        }
    }
}

/// Returns `(segment_pcts, dot_pct)` for the minimap scrollbar.
///
/// Each segment gets at least `MIN_PCT`% of the bar width (so tiny categories remain
/// clickable), then all widths are renormalized to sum to 100%.  The dot position is
/// interpolated through those same adjusted widths so it always lines up with the
/// visual segment boundaries.
fn minimap_segments(
    categories: &[Category],
    heights: &[(Category, f64)],
    scroll_top: f64,
) -> (Vec<(Category, f64)>, f64) {
    const MIN_PCT: f64 = 6.0;
    let n = categories.len();
    let total_h: f64 = heights.iter().map(|(_, v)| v).sum();
    let equal_pct = 100.0 / n as f64;

    let bumped: Vec<f64> = categories
        .iter()
        .map(|&cat| {
            let raw = if total_h > 0.0 {
                heights
                    .iter()
                    .find(|(c, _)| *c == cat)
                    .map_or(equal_pct, |(_, v)| v / total_h * 100.0)
            } else {
                equal_pct
            };
            raw.max(MIN_PCT)
        })
        .collect();
    let bumped_sum: f64 = bumped.iter().sum();
    let adj: Vec<f64> = bumped.iter().map(|&p| p / bumped_sum * 100.0).collect();

    let dot = if total_h > 0.0 {
        let mut cum_content = 0.0f64;
        let mut cum_adj = 0.0f64;
        let mut result = 100.0f64;
        for (i, &cat) in categories.iter().enumerate() {
            let height = heights
                .iter()
                .find(|(c, _)| *c == cat)
                .map_or(0.0, |(_, v)| *v);
            if scroll_top < cum_content + height || i + 1 == n {
                let t = if height > 0.0 {
                    ((scroll_top - cum_content) / height).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                result = (cum_adj + t * adj[i]).clamp(0.0, 100.0);
                break;
            }
            cum_content += height;
            cum_adj += adj[i];
        }
        result
    } else {
        0.0
    };

    (categories.iter().copied().zip(adj).collect(), dot)
}

#[component]
fn CategoryScrollBar(
    categories: Vec<Category>,
    scroll_top: Signal<f64>,
    cat_heights: Vec<(Category, f64)>,
) -> Element {
    if categories.is_empty() {
        return rsx! {};
    }

    let n = categories.len();
    let (segment_pcts, dot_pct) = minimap_segments(&categories, &cat_heights, *scroll_top.read());

    rsx! {
        div {
            id: "category-scroll-bar",
            position: "relative",
            display: "flex",
            align_items: "stretch",
            height: "1.2rem",

            div {
                position: "absolute",
                top: "50%",
                left: "calc({dot_pct:.2}% - 0.6rem)",
                width: "1.2rem",
                height: "1.2rem",
                transform: "translateY(-50%)",
                background_color: "color-mix(in srgb, var(--accent) 60%, transparent)",
                backdrop_filter: "blur(1px)",
                border_radius: "9999px",
                pointer_events: "none",
                z_index: "1",
                transition: "left 0.06s ease",
            }

            for (i, &(category, pct)) in segment_pcts.iter().enumerate() {
                div {
                    flex: "0 0 {pct:.4}%",
                    min_width: "0",
                    overflow: "hidden",
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    background_color: "rgba(255,255,255,0.08)",
                    border_right: if i + 1 < n { "2px solid var(--base)" } else { "none" },
                    border_radius: if i == 0 { "0.3rem 0 0 0.3rem" } else if i + 1 == n { "0 0.3rem 0.3rem 0" } else { "0" },
                    color: "var(--text)",
                    cursor: "pointer",
                    title: "{category}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        if let (Some(scroll_el), Some(cat_el)) = (
                            get_element("article-scroll-container"),
                            get_element(&format!("category-group-{category}")),
                        ) {
                            let sr = scroll_el.get_bounding_client_rect();
                            let cr = cat_el.get_bounding_client_rect();
                            let new_top =
                                scroll_el.scroll_top() as f64 + cr.top() - sr.top() - 8.0;
                            scroll_el.set_scroll_top(new_top.max(0.0) as i32);
                        }
                    },
                    span {
                        position: "relative",
                        z_index: "2",
                        pointer_events: "none",
                        filter: "drop-shadow(0.5px 0.5px 1px rgba(0,0,0,0.6))",
                        {category_icon(category, 13)}
                    }
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
            border_bottom: "{CATEGORY_BORDER_PX}px solid var(--base)",
            background_color: "rgba(54, 58, 79, 0.15)",

            div { width: LABEL_WIDTH, background_color: "var(--surface0)",
                div {
                    position: "sticky",
                    top: "0",
                    width: "100%",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    padding: "0.8rem",
                    gap: "0.75rem",
                    color: "var(--text)",
                    {category_icon(category, 24)}
                    div {
                        writing_mode: "vertical-rl",
                        transform: "rotate(180deg)",
                        font_size: "1.2rem",
                        font_weight: "800",
                        color: "var(--text)",
                        text_transform: "uppercase",
                        letter_spacing: "0.12em",
                        "{category}"
                    }
                }
            }
            div {
                flex: "1",
                display: "flex",
                flex_direction: "column",
                gap: "{ARTICLE_GAP_PX}px",
                padding: "{ARTICLE_GAP_PX}px",
                {children}
            }
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
        "color-mix(in srgb, var(--accent) 22%, var(--crust))"
    } else {
        "var(--crust)"
    };
    let scale = if is_pressed {
        "scale(0.97)"
    } else if is_hovered {
        "scale(1.02)"
    } else {
        "scale(1)"
    };
    let shadow = if is_hovered {
        "1px 2px 6px rgba(0,0,0,0.8)"
    } else {
        "0.5px 1px 4px rgba(0,0,0,0.6)"
    };
    let border_color = if is_hovered {
        "rgba(255,255,255,0.3)"
    } else {
        "rgba(255,255,255,0.18)"
    };

    rsx! {
        div {
            style: "content-visibility:auto;contain-intrinsic-size:auto {ARTICLE_HEIGHT_PX}px",
            cursor: "pointer",
            border_radius: "0.625rem",
            box_shadow: shadow,
            transform: scale,
            transition: "transform 0.15s ease-out, box-shadow 0.2s ease-out",
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
            div {
                id: "article-{id}",
                position: "relative",
                border_radius: "0.625rem",
                height: "{ARTICLE_HEIGHT_PX}px",
                overflow: "hidden",
                background_color: bg,
                border: "2px solid {border_color}",
                transition: "border-color 0.2s ease-out, background-color 0.2s ease-out",

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
                        "{title}"
                    }
                    p {
                        font_size: "0.75rem",
                        color: "rgba(255,255,255,0.85)",
                        line_height: "1.4",
                        font_weight: "600",
                        margin: "0",
                        overflow: "hidden",
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
}
