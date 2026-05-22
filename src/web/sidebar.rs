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
pub const ARTICLE_GAP_PX: u32 = 12;

/// Separator border thickness below each category group.
const CATEGORY_BORDER_PX: u32 = 12;

/// Minimum size of a category in the horizontal scrollbar.
const MIN_CATEGORY_PCT: f64 = 6.0;

/// Styles for the sidebar.
pub const SIDEBAR_STYLES: &str = "
    @keyframes bubble-pop {
        0% { transform: scale(1, 1); }
        35% { transform: scale(1.2, 0.8); }
        65% { transform: scale(0.9, 1.1); }
        100% { transform: scale(1, 1); }
    }
    .tab-bubble-active {
        animation: bubble-pop 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
";

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
pub fn Sidebar(tab: String, selected_id: Option<Uuid>) -> Element {
    let mut articles: Signal<Vec<ArticleData>> = use_context();

    let (counts, all_groups) = use_memo(use_reactive!(|articles| {
        let (mut counts, mut groups) = (
            (0, 0, 0),
            BTreeMap::<ArticleStatus, BTreeMap<Category, Vec<ArticleData>>>::new(),
        );
        for a in &articles() {
            *match a.status {
                ArticleStatus::New => &mut counts.0,
                ArticleStatus::Stored => &mut counts.1,
                ArticleStatus::Binned => &mut counts.2,
            } += 1;
            groups
                .entry(a.status)
                .or_default()
                .entry(a.article.category)
                .or_default()
                .push(a.clone());
        }
        (counts, groups)
    }))();

    let current_status = match tab.as_str() {
        "stored" => ArticleStatus::Stored,
        "binned" => ArticleStatus::Binned,
        _ => ArticleStatus::New,
    };

    let current_groups = all_groups.get(&current_status).cloned().unwrap_or_default();
    let scroll_top_val = use_signal(|| 0.0);

    rsx! {
        div {
            width: SIDEBAR_WIDTH,
            display: "flex",
            flex_direction: "column",
            overflow: "hidden",
            padding: "10px",

            div {
                id: "sidebar-header",
                display: "flex",
                flex_direction: "column",
                gap: "0.8rem",
                padding: "0.8rem",

                div {
                    display: "flex",
                    justify_content: "space-between",
                    align_items: "center",
                    h1 {
                        font_size: "2rem",
                        font_weight: "800",
                        color: "var(--text)",
                        text_transform: "uppercase",
                        margin: "0",
                        "Quivrs"
                    }
                    RefreshButton {
                        title: "Re-classify all articles",
                        onclick: move |_| async move {
                            let ids = articles().iter().map(|a| a.id).collect();
                            if reclassify_articles(ids).await.is_ok()
                                && let Ok(new_articles) = get_user_articles().await
                            {
                                articles.set(new_articles);
                            }
                        },
                    }
                }

                TabNav {
                    tab: tab.clone(),
                    new_count: counts.0,
                    stored_count: counts.1,
                    binned_count: counts.2,
                }
                CategoryScrollbar {
                    groups: current_groups,
                    status: current_status,
                    scroll_top: scroll_top_val,
                }
            }

            div {
                id: "article-scroll-container",
                flex: "1",
                overflow: "hidden",
                position: "relative",
                background_color: "var(--mantle)",
                border_radius: "20px 40px 0px 0px",

                div {
                    display: "flex",
                    width: "100%",
                    height: "100%",
                    transition: "transform 1.4s cubic-bezier(0.4, 0, 0.2, 1)",
                    transform: "translateX(calc(-{current_status as usize} * 100% - {current_status as usize} * 40px))",
                    will_change: "transform",

                    for status in [ArticleStatus::Stored, ArticleStatus::New, ArticleStatus::Binned] {
                        StatusLane {
                            key: "{status}",
                            status,
                            active: current_status == status,
                            groups: all_groups.get(&status).cloned().unwrap_or_default(),
                            selected_id,
                            tab: tab.clone(),
                            scroll_top_val,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatusLane(
    status: ArticleStatus,
    active: bool,
    groups: BTreeMap<Category, Vec<ArticleData>>,
    selected_id: Option<Uuid>,
    tab: String,
    mut scroll_top_val: Signal<f64>,
) -> Element {
    rsx! {
        div {
            flex: "0 0 100%",
            height: "100%",
            overflow_y: "auto",
            margin_right: if status == ArticleStatus::Binned { "0" } else { "40px" },
            onscroll: move |e: ScrollEvent| {
                if active {
                    scroll_top_val.set(e.scroll_top());
                }
            },

            for (category, items) in groups {
                CategoryGroup { key: "{category}-{status}", category, status,
                    for a in items {
                        ArticleItem {
                            key: "{a.id}",
                            id: a.id,
                            rating: a.rating,
                            article: a.article,
                            selected: selected_id,
                            tab: tab.clone(),
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TabNav(tab: String, new_count: usize, stored_count: usize, binned_count: usize) -> Element {
    let bubble_left = match tab.as_str() {
        "stored" => "0%",
        "new" => "33.33%",
        _ => "66.66%",
    };

    let mut trigger_anim = use_signal(|| false);
    use_memo(use_reactive!(|tab| {
        let _ = tab;
        trigger_anim.set(true);
    }));

    rsx! {
        div {
            position: "relative",
            display: "flex",
            background_color: "var(--surface0)",
            border: "2px solid var(--surface1)",
            border_radius: "2rem",

            div {
                class: if trigger_anim() { "tab-bubble-active" },
                onanimationend: move |_| trigger_anim.set(false),
                position: "absolute",
                top: "0",
                bottom: "0",
                left: "{bubble_left}",
                width: "33.33%",
                background_color: "var(--accent)",
                border_radius: "2rem",
                transition: "left 0.45s cubic-bezier(0.34, 1.56, 0.64, 1)",
                pointer_events: "none",
                z_index: "1",
                will_change: "left, transform",
            }

            for (slug, label, count) in [
                ("stored", "Stored", stored_count),
                ("new", "New", new_count),
                ("binned", "Binned", binned_count),
            ]
            {
                TabButton {
                    slug,
                    label,
                    count,
                    active: tab == slug,
                }
            }
        }
    }
}

#[component]
fn TabButton(slug: &'static str, label: &'static str, count: usize, active: bool) -> Element {
    let nav = use_navigator();
    let mut pressed = use_signal(|| false);
    let mut hovered = use_signal(|| false);

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
            border: "none",
            background_color: "transparent",
            cursor: "pointer",
            color: "var(--text)",
            font_size: "0.875rem",
            font_weight: "700",
            text_shadow: "0.5px 0.5px 1px rgba(0,0,0,0.6)",
            transform: if hovered() { "scale(1.05)" } else if pressed() { "scale(0.94)" } else { "scale(1)" },
            transition: "transform 0.1s ease",
            // OPTIMIZATION: Micro-button scale interactions drop layout overhead.
            will_change: "transform",
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
                background_color: "rgba(0,0,0,0.2)",
                border_radius: "9999px",
                padding: "0.125rem 0.4rem",
                font_size: "0.72rem",
                text_shadow: "none",
                "{count}"
            }
        }
    }
}

#[component]
fn CategoryScrollbar(
    groups: BTreeMap<Category, Vec<ArticleData>>,
    status: ArticleStatus,
    scroll_top: Signal<f64>,
) -> Element {
    if groups.is_empty() {
        return rsx! {};
    }

    let cat_heights: Vec<f64> = groups
        .values()
        .map(|items| {
            (items.len() as f64).mul_add(
                f64::from(ARTICLE_HEIGHT_PX + ARTICLE_GAP_PX),
                f64::from(ARTICLE_GAP_PX + CATEGORY_BORDER_PX),
            )
        })
        .collect();

    let segment_pcts = use_memo(use_reactive!(|cat_heights| {
        let total_h: f64 = cat_heights.iter().sum();
        if total_h <= 0.0 {
            return vec![0.0; cat_heights.len()];
        }
        let bumped: Vec<f64> = cat_heights
            .iter()
            .map(|h| (*h / total_h * 100.0).max(MIN_CATEGORY_PCT))
            .collect();
        let sum: f64 = bumped.iter().sum();
        bumped.into_iter().map(|p| p / sum * 100.0).collect()
    }));

    let dot_pct = use_memo(use_reactive!(|scroll_top, cat_heights, segment_pcts| {
        let st = scroll_top();
        let (mut cur_h, mut cur_adj) = (0.0, 0.0);
        let pcts = segment_pcts();

        for (i, h) in cat_heights.iter().enumerate() {
            if st < cur_h + *h || i == cat_heights.len() - 1 {
                let t = if *h > 0.0 { (st - cur_h) / *h } else { 0.0 };
                return t.clamp(0.0, 1.0).mul_add(pcts[i], cur_adj);
            }
            cur_h += *h;
            cur_adj += pcts[i];
        }
        0.0
    }));

    rsx! {
        div {
            id: "category-scroll-bar",
            position: "relative",
            display: "flex",
            height: "1.4rem",
            background_color: "var(--surface0)",
            border: "2px solid var(--surface1)",
            border_radius: "9999px",
            align_items: "center",
            overflow: "hidden",

            div {
                position: "absolute",
                left: "{dot_pct:.2}%",
                height: "100%",
                aspect_ratio: "1.2",
                background_color: "var(--accent)",
                border_radius: "9999px",
                transition: "left 0.45s cubic-bezier(0.175, 0.885, 0.32, 1.1)",
                will_change: "left",
            }

            for (i, &category) in groups.keys().enumerate() {
                div {
                    key: "{category}",
                    flex: "0 0 {segment_pcts()[i]:.4}%",
                    display: "flex",
                    height: "100%",
                    align_items: "center",
                    justify_content: "center",
                    position: "relative",
                    cursor: "pointer",
                    title: "{category}",
                    z_index: "1",
                    transition: "flex-basis 0.45s cubic-bezier(0.175, 0.885, 0.32, 1.1)",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        if let Some(el) = web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| {
                                d.get_element_by_id(&format!("category-group-{category}-{status}"))
                            })
                        {
                            el.scroll_into_view();
                        }
                    },

                    if i + 1 < groups.len() {
                        div {
                            position: "absolute",
                            right: "0",
                            top: "15%",
                            bottom: "15%",
                            width: "1.5px",
                            transform: "translateX(50%)",
                            background_color: "var(--overlay0)",
                            filter: "blur(0.5px)",
                            z_index: "-1",
                        }
                    }

                    span { filter: "drop-shadow(0.5px 0.5px 1px rgba(0,0,0,0.6))",
                        {category_icon(category, 13)}
                    }
                }
            }
        }
    }
}

#[component]
fn CategoryGroup(category: Category, status: ArticleStatus, children: Element) -> Element {
    rsx! {
        div {
            id: "category-group-{category}-{status}",
            display: "flex",
            border_bottom: "{CATEGORY_BORDER_PX}px solid var(--mantle)",

            div {
                width: LABEL_WIDTH,
                background_color: "var(--surface0)",
                border_radius: "20px 0px 0px 20px",
                div {
                    position: "sticky",
                    top: "0",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    padding: "0.8rem",
                    gap: "0.75rem",
                    {category_icon(category, 24)}
                    div {
                        writing_mode: "vertical-rl",
                        transform: "rotate(180deg)",
                        font_size: "1.2rem",
                        font_weight: "800",
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
                background_color: "var(--accent)",
                border_radius: "0px 40px 40px 0px",
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

    let title = article.display_title();
    let description = article.display_description();
    let description_truncated = description.chars().take(200).collect::<String>();

    let hero_image = article.sources.iter().find_map(|s| {
        s.images
            .first()?
            .split('|')
            .next()
            .filter(|u| !u.is_empty())
    });

    let time_ago = use_memo(move || {
        let d = chrono::Utc::now().signed_duration_since(article.published);
        if d.num_days() > 0 {
            format!("{}d", d.num_days())
        } else if d.num_hours() > 0 {
            format!("{}h", d.num_hours())
        } else {
            format!("{}m", d.num_minutes().max(1))
        }
    });

    rsx! {
        div {
            id: "article-{id}",
            height: "{ARTICLE_HEIGHT_PX}px",
            cursor: "pointer",
            border_radius: "2rem",
            border: "2px solid var(--surface1)",
            box_shadow: if hovered() { "1px 2px 6px rgba(0,0,0,0.8)" } else { "0.5px 1px 4px rgba(0,0,0,0.6)" },
            transition: "box-shadow 0.2s ease",

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
                width: "100%",
                height: "100%",
                border_radius: "calc(2rem - 2px)",
                overflow: "hidden",
                display: "flex",
                flex_direction: "column",
                // Fixes clipping issues
                style: "-webkit-mask-image: -webkit-radial-gradient(white, black); mask-image: radial-gradient(white, black);",
                transform: "translateZ(0)",

                div { flex: "1", overflow: "hidden", position: "relative",

                    if let Some(img_url) = hero_image {
                        img {
                            src: "{img_url}",
                            width: "100%",
                            height: "100%",
                            object_fit: "cover",
                            object_position: "center 35%",
                            transform: if hovered() { "scale(1.03)" } else { "scale(1)" },
                            transition: "transform 0.4s ease",
                            will_change: "transform",
                            decoding: "async",
                        }
                    }

                    if let Some(r) = rating {
                        div {
                            position: "absolute",
                            top: "0",
                            left: "0",
                            width: "70px",
                            height: "50px",
                            clip_path: "polygon(0 0, 100% 0, 0 100%)",
                            background_color: "color-mix(in srgb, {rating_color(r)} 70%, transparent)",
                            backdrop_filter: "blur(8px)",
                        }
                    }
                }

                div {
                    padding: "0.75rem",
                    background_color: if is_selected { "color-mix(in srgb, var(--accent) 40%, var(--mantle))" } else { "var(--mantle)" },
                    text_shadow: "0.5px 0.5px 1px rgba(0,0,0,0.6)",
                    transition: "background 0.5s ease",
                    h3 {
                        font_size: "1rem",
                        font_weight: "700",
                        color: "var(--text)",
                        margin: "0 0 0.4rem 0",
                        "{title}"
                    }
                    p {
                        font_size: "0.75rem",
                        color: "var(--subtext1)",
                        font_weight: "600",
                        margin: "0",
                        span {
                            font_size: "0.65rem",
                            font_weight: "900",
                            text_transform: "uppercase",
                            margin_right: "0.4rem",
                            "{time_ago}"
                        }
                        "{description_truncated}"
                    }
                }
            }
        }
    }
}
