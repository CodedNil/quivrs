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

    let processed_data = use_memo(move || {
        let data = articles.read();
        let mut counts = (0, 0, 0);
        let mut groups: BTreeMap<ArticleStatus, BTreeMap<Category, Vec<ArticleData>>> =
            BTreeMap::new();

        for a in data.iter() {
            match a.status {
                ArticleStatus::New => counts.0 += 1,
                ArticleStatus::Stored => counts.1 += 1,
                ArticleStatus::Binned => counts.2 += 1,
            }
            groups
                .entry(a.status)
                .or_default()
                .entry(a.article.category)
                .or_default()
                .push(a.clone());
        }
        (counts, groups)
    });

    let counts = processed_data.read().0;
    let current_status = match tab.as_str() {
        "stored" => ArticleStatus::Stored,
        "binned" => ArticleStatus::Binned,
        _ => ArticleStatus::New,
    };

    let current_groups = processed_data
        .read()
        .1
        .get(&current_status)
        .cloned()
        .unwrap_or_default();

    let cat_heights: Vec<(Category, f64)> = current_groups
        .iter()
        .map(|(&cat, items)| {
            let h = (items.len() as f64).mul_add(
                f64::from(ARTICLE_HEIGHT_PX + ARTICLE_GAP_PX),
                f64::from(ARTICLE_GAP_PX + CATEGORY_BORDER_PX),
            );
            (cat, h)
        })
        .collect();

    let mut scroll_top_val = use_signal(|| 0.0);
    let x_offset = match tab.as_str() {
        "stored" => "0%",
        "new" => "-33.333%",
        _ => "-66.666%",
    };

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
                z_index: "10",

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
                    tab: tab.clone(),
                    new_count: counts.0,
                    stored_count: counts.1,
                    binned_count: counts.2,
                }
                CategoryScrollbar {
                    categories: current_groups.keys().copied().collect(),
                    scroll_top: scroll_top_val,
                    cat_heights,
                }
            }

            div {
                id: "article-scroll-container",
                flex: "1",
                overflow: "hidden",
                position: "relative",
                div {
                    display: "flex",
                    width: "300%",
                    height: "100%",
                    transition: "transform 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
                    transform: "translateX({x_offset})",

                    for status in [ArticleStatus::Stored, ArticleStatus::New, ArticleStatus::Binned] {
                        div {
                            key: "{status}",
                            width: "33.333%",
                            height: "100%",
                            overflow_y: "auto",
                            onscroll: move |e: ScrollEvent| {
                                if status == current_status {
                                    scroll_top_val.set(e.scroll_top());
                                }
                            },
                            if status == current_status {
                                for (category, items) in current_groups.iter() {
                                    CategoryGroup {
                                        key: "{category}",
                                        category: *category,
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
                            }
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
        "stored" => "0",
        "new" => "calc((100% - 0.5rem) / 3 + 0.25rem)",
        _ => "calc((100% - 0.5rem) * 2 / 3 + 0.5rem)",
    };

    let mut trigger_anim = use_signal(|| false);
    use_memo(use_reactive!(|tab| {
        let _ = tab;
        trigger_anim.set(true);
    }));

    rsx! {
        div { position: "relative", display: "flex", gap: "0.25rem",
            div {
                class: if trigger_anim() { "tab-bubble-active" },
                onanimationend: move |_| trigger_anim.set(false),
                position: "absolute",
                top: "0",
                bottom: "0",
                left: "{bubble_left}",
                width: "calc((100% - 0.5rem) / 3)",
                background_color: "var(--accent)",
                border_radius: "2rem",
                transition: "left 0.45s cubic-bezier(0.34, 1.56, 0.64, 1)",
                pointer_events: "none",
                z_index: "1",
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
            border_radius: "9999px",
            border: "none",
            background_color: if active { "transparent" } else { "var(--surface0)" },
            cursor: "pointer",
            color: "var(--text)",
            font_size: "0.875rem",
            font_weight: "700",
            transform: if pressed() { "scale(0.94)" } else { "scale(1)" },
            transition: "background-color 0.15s ease, transform 0.1s ease",
            onmousedown: move |_| pressed.set(true),
            onmouseup: move |_| pressed.set(false),
            onmouseleave: move |_| pressed.set(false),
            onclick: move |_| {
                nav.push(Route::TabHome {
                    tab: slug.to_string(),
                });
            },
            "{label}"
            span {
                background_color: "rgba(255,255,255,0.12)",
                border_radius: "9999px",
                padding: "0.125rem 0.4rem",
                font_size: "0.72rem",
                "{count}"
            }
        }
    }
}

#[component]
fn CategoryScrollbar(
    categories: Vec<Category>,
    scroll_top: Signal<f64>,
    cat_heights: Vec<(Category, f64)>,
) -> Element {
    if categories.is_empty() {
        return rsx! {};
    }

    let total_h: f64 = cat_heights.iter().map(|(_, v)| v).sum();

    let segment_pcts: Vec<f64> = {
        let bumped: Vec<f64> = cat_heights
            .iter()
            .map(|(_, h)| {
                if *h > 0.0 {
                    (*h / total_h * 100.0).max(MIN_CATEGORY_PCT)
                } else {
                    0.0
                }
            })
            .collect();
        let sum: f64 = bumped.iter().sum();
        bumped.into_iter().map(|p| p / sum * 100.0).collect()
    };

    let dot_pct = {
        let st = *scroll_top.read();
        let mut cumulative_content = 0.0;
        let mut cumulative_adj = 0.0;
        let mut pos = 0.0;
        for (i, (_, h)) in cat_heights.iter().enumerate() {
            if st < cumulative_content + h || i == categories.len() - 1 {
                let t = if *h > 0.0 {
                    ((st - cumulative_content) / h).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                pos = cumulative_adj + t * segment_pcts[i];
                break;
            }
            cumulative_content += h;
            cumulative_adj += segment_pcts[i];
        }
        pos
    };

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
                aspect_ratio: "1",
                background_color: "var(--accent)",
                border_radius: "9999px",
                filter: "blur(2px)",
                z_index: "0",
                transition: "left 0.45s cubic-bezier(0.175, 0.885, 0.32, 1.1)",
            }

            for (i, &category) in categories.iter().enumerate() {
                div {
                    key: "{category}",
                    flex: "0 0 {segment_pcts[i]:.4}%",
                    display: "flex",
                    height: "100%",
                    align_items: "center",
                    justify_content: "center",
                    position: "relative",
                    cursor: "pointer",
                    title: "{category}",
                    z_index: "1",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        if let Some(el) = web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.get_element_by_id(&format!("category-group-{category}")))
                        {
                            el.scroll_into_view();
                        }
                    },

                    // Visual separator
                    if i + 1 < categories.len() {
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
fn CategoryGroup(category: Category, children: Element) -> Element {
    rsx! {
        div {
            id: "category-group-{category}",
            display: "flex",
            border_bottom: "{CATEGORY_BORDER_PX}px solid var(--base)",
            background_color: "var(--surface0)",

            div { width: LABEL_WIDTH, background_color: "var(--surface0)",
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
            .filter(|url| !url.is_empty())
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

    rsx! {
        div {
            style: "content-visibility:auto;contain-intrinsic-size:auto {ARTICLE_HEIGHT_PX}px;",
            cursor: "pointer",
            border_radius: "0.625rem",
            transform: if pressed() { "scale(0.98)" } else if hovered() { "scale(1.01)" } else { "scale(1)" },
            box_shadow: if hovered() { "1px 2px 6px rgba(0,0,0,0.8)" } else { "0.5px 1px 4px rgba(0,0,0,0.6)" },
            transition: "transform 0.2s ease, box-shadow 0.2s ease",
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
                background_color: if is_selected { "color-mix(in srgb, var(--accent) 22%, var(--crust))" } else { "var(--crust)" },
                border: "2px solid",
                border_color: if hovered() { "rgba(255,255,255,0.3)" } else { "rgba(255,255,255,0.18)" },
                transition: "border-color 0.2s ease, background-color 0.2s ease",

                if let Some(img_url) = hero_image {
                    img {
                        src: "{img_url}",
                        position: "absolute",
                        width: "100%",
                        height: "100%",
                        object_fit: "cover",
                        object_position: "center 35%",
                        transform: if hovered() { "scale(1.05)" } else { "scale(1)" },
                        transition: "transform 0.4s ease",
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
                        margin: "0 0 0.4rem 0",
                        "{title}"
                    }
                    p {
                        font_size: "0.75rem",
                        color: "rgba(255,255,255,0.85)",
                        font_weight: "600",
                        margin: "0",
                        span {
                            font_size: "0.65rem",
                            color: "rgba(255,255,255,0.7)",
                            font_weight: "900",
                            text_transform: "uppercase",
                            margin_right: "0.6rem",
                            "{time_ago}"
                        }
                        "{description_truncated}"
                    }
                }
            }
        }
    }
}
