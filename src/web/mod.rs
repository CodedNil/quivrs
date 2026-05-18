mod components;

use crate::{
    shared::{
        ArticleData, ArticleEntry, ArticleStatus, Category, Rating, StoredArticle,
        server_functions::{get_all_item_ratings, get_user_articles},
    },
    web::components::{
        CategoryGroup, RatingPill, SectionHeader, clean_url, rating_button, rating_color,
        render_section, status_button, style,
    },
};
use dioxus::prelude::*;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

pub fn app() -> Element {
    let mut articles: Signal<Vec<ArticleData>> = use_signal(Vec::new);
    let mut item_ratings: Signal<HashMap<String, Rating>> = use_signal(HashMap::new);
    let selected: Signal<Option<Uuid>> = use_signal(|| None);

    use_resource(move || async move {
        if let Ok(data) = get_user_articles().await {
            *articles.write() = data;
        }
    });
    use_resource(move || async move {
        if let Ok(data) = get_all_item_ratings().await {
            *item_ratings.write() = data;
        }
    });

    rsx! {
        document::Title { "Quivrs" }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap",
        }
        // <https://raw.githubusercontent.com/chriskempson/base16/refs/heads/main/styling.md>
        // Colours base00 to base07 are typically variations of a shade and run from darkest to lightest. These colours are used for foreground and background, status bars, line highlighting and such. colours base08 to base0F are typically individual colours used for types, operators, names and variables. In order to create a dark theme, colours base00 to base07 should span from dark to light. For a light theme, these colours should span from light to dark.
        document::Style {
            ":root {{
                --base00: #24273a; --base01: #1e2030; --base02: #363a4f; --base03: #494d64;
                --base04: #5b6078; --base05: #cad3f5; --base06: #f4dbd6; --base07: #b7bdf8;
                --base08: #ed8796; --base09: #f5a97f; --base0a: #eed49f; --base0b: #a6da95;
                --base0c: #8bd5ca; --base0d: #8aadf4; --base0e: #c6a0f6; --base0f: #f0c6c6;
            }}
            body, button, input, select, textarea {{
                font-family: 'Inter', system-ui, -apple-system, sans-serif;
                margin: 0; padding: 0;
            }}
            *, *::before, *::after {{ box-sizing: border-box; }}"
        }
        div {
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            background_color: "var(--base00)",
            color: "var(--base05)",

            aside {
                width: "27rem",
                border_right: "1px solid var(--base02)",
                display: "flex",
                flex_direction: "column",
                background_color: style::SIDEBAR_BG,

                div {
                    padding: "1rem 1.25rem",
                    border_bottom: "1px solid var(--base02)",
                    h1 {
                        font_size: "0.75rem",
                        font_weight: "700",
                        letter_spacing: "0.1em",
                        color: "var(--base05)",
                        text_transform: "uppercase",
                        margin: "0",
                        "Quivrs"
                    }
                }

                div { overflow_y: "auto", flex: "1",
                    {
                        let data = articles.read();
                        if data.is_empty() {
                            rsx! {
                                p { padding: "1.25rem", font_size: "0.75rem", color: "var(--base05)", "Loading..." }
                            }
                        } else {
                            let mut new_v = Vec::new();
                            let mut stored_v = Vec::new();
                            let mut binned_v = Vec::new();
                            for (id, status, rating, article) in data.iter() {
                                if let Some(entry) = &article.entry {
                                    match status {
                                        ArticleStatus::New => {
                                            new_v.push((*id, *rating, article, entry));
                                        }
                                        ArticleStatus::Stored => {
                                            stored_v.push((*id, *rating, article, entry));
                                        }
                                        ArticleStatus::Binned => {
                                            binned_v.push((*id, *rating, article, entry));
                                        }
                                    }
                                }
                            }
                            rsx! {
                                {sidebar_section("New", new_v, selected, true)}
                                {sidebar_section("Stored", stored_v, selected, false)}
                                {sidebar_section("Binned", binned_v, selected, false)}
                            }
                        }
                    }
                }
            }

            main { flex: "1", overflow_y: "auto",
                {
                    selected
                        .read()
                        .map_or_else(
                            || rsx! {
                                div {
                                    display: "flex",
                                    align_items: "center",
                                    justify_content: "center",
                                    height: "100%",
                                    color: "var(--base04)",
                                    font_size: "0.875rem",
                                    "Select an article to read"
                                }
                            },
                            |id| {
                                let found = articles
                                    .read()
                                    .iter()
                                    .find(|(i, _, _, _)| *i == id)
                                    .map(|(_, s, r, a)| (*s, *r, a.clone()));
                                match found {
                                    Some((status, rating, art)) => rsx! {
                                        article_detail {
                                            article: art,
                                            status,
                                            rating,
                                            articles,
                                            item_ratings,
                                        }
                                    },
                                    None => rsx! {},
                                }
                            },
                        )
                }
            }
        }
    }
}

fn sidebar_section(
    label: &str,
    items: Vec<(Uuid, Option<Rating>, &StoredArticle, &ArticleEntry)>,
    selected: Signal<Option<Uuid>>,
    group_by_category: bool,
) -> Element {
    let total_items = items.len();

    let content = if group_by_category {
        let mut groups: BTreeMap<Category, Vec<_>> = BTreeMap::new();
        for (id, rating, article, entry) in items {
            groups
                .entry(entry.category)
                .or_default()
                .push((id, rating, article));
        }
        rsx! {
            for (category, cat_items) in groups {
                CategoryGroup { category,
                    for (id, rating, article) in cat_items {
                        article_item {
                            key: "{id}",
                            id,
                            rating,
                            article: article.clone(),
                            selected,
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            for (id, rating, article, _) in items {
                article_item {
                    key: "{id}",
                    id,
                    rating,
                    article: article.clone(),
                    selected,
                }
            }
        }
    };

    rsx! {
        SectionHeader { label: label.to_string(), count: total_items }
        {content}
    }
}

#[component]
fn article_item(
    id: Uuid,
    rating: Option<Rating>,
    article: StoredArticle,
    mut selected: Signal<Option<Uuid>>,
) -> Element {
    let is_selected = *selected.read() == Some(id);
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);

    let Some(entry) = &article.entry else {
        return rsx! {};
    };

    let hero_image = article.sources.iter().find_map(|s| s.image.clone());

    let bg = if is_selected {
        style::CARD_BG_SELECTED
    } else {
        style::CARD_BG_IDLE
    };
    let shadow = if *pressed.read() {
        style::CARD_SHADOW_ACTIVE
    } else if *hovered.read() {
        style::CARD_SHADOW_HOVER
    } else if is_selected {
        style::CARD_SHADOW_SELECTED
    } else {
        style::CARD_SHADOW_IDLE
    };
    let scale = if *pressed.read() {
        "scale(0.97)"
    } else if *hovered.read() {
        "scale(1.01)"
    } else {
        "scale(1)"
    };

    rsx! {
        div {
            cursor: "pointer",
            position: "relative",
            border_radius: style::RADIUS_CARD,
            margin: "0.375rem 0.5rem",
            overflow: "hidden",
            background_color: bg,
            box_shadow: shadow,
            transform: scale,
            transition: style::TRANSITION_CARD,
            onclick: move |_| selected.set(Some(id)),
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| {
                hovered.set(false);
                pressed.set(false);
            },
            onmousedown: move |_| pressed.set(true),
            onmouseup: move |_| pressed.set(false),

            if let Some(img_url) = hero_image {
                img {
                    src: "{img_url}",
                    alt: "",
                    width: "100%",
                    style: "height: 144px; object-fit: cover; display: block;",
                }
            }

            // Rating notch: triangle clipped to top-left corner
            if let Some(r) = rating {
                div {
                    position: "absolute",
                    top: "0",
                    left: "0",
                    width: "24px",
                    height: "24px",
                    background_color: rating_color(r),
                    style: "clip-path: polygon(0 0, 100% 0, 0 100%); z-index: 1; filter: drop-shadow(1px 2px 3px rgba(0,0,0,0.55));",
                }
            }

            div { padding: "0.625rem 0.75rem",
                h3 {
                    font_size: "0.75rem",
                    font_weight: "600",
                    color: "var(--base05)",
                    line_height: "1.375",
                    margin: "0 0 0.2rem 0",
                    "{entry.title}"
                }
                p {
                    font_size: "0.67rem",
                    color: "var(--base05)",
                    line_height: "1.5",
                    margin: "0",
                    "{entry.description}"
                }
            }
        }
    }
}

#[component]
fn article_detail(
    article: StoredArticle,
    status: ArticleStatus,
    rating: Option<Rating>,
    articles: Signal<Vec<ArticleData>>,
    item_ratings: Signal<HashMap<String, Rating>>,
) -> Element {
    let id = article.id;

    rsx! {
        article {
            max_width: "42rem",
            margin_left: "auto",
            margin_right: "auto",
            padding: "2rem 2.5rem 5rem",

            div {
                display: "flex",
                align_items: "center",
                justify_content: "space-between",
                padding_bottom: "1rem",
                margin_bottom: "1.5rem",
                border_bottom: "1px solid var(--base02)",
                div { display: "flex", gap: "0.375rem",
                    {status_button("New", ArticleStatus::New, status, id, articles)}
                    {status_button("Store", ArticleStatus::Stored, status, id, articles)}
                    {status_button("Bin", ArticleStatus::Binned, status, id, articles)}
                }
                div { display: "flex", gap: "0.25rem",
                    {rating_button("Hate", Rating::Hated, rating, id, articles)}
                    {rating_button("Dislike", Rating::Disliked, rating, id, articles)}
                    {rating_button("Neutral", Rating::Neutral, rating, id, articles)}
                    {rating_button("Like", Rating::Liked, rating, id, articles)}
                    {rating_button("Love", Rating::Loved, rating, id, articles)}
                }
            }

            if let Some(entry) = &article.entry {
                h1 {
                    font_size: "1.5rem",
                    font_weight: "700",
                    line_height: "1.25",
                    color: "var(--base05)",
                    margin: "0 0 0.375rem 0",
                    "{entry.title}"
                }
                p {
                    font_size: "0.7rem",
                    color: "var(--base05)",
                    margin: "0 0 1rem 0",
                    {article.updated_at.format("%Y-%m-%d %H:%M UTC").to_string()}
                }
                p {
                    font_size: "0.875rem",
                    color: "var(--base05)",
                    line_height: "1.625",
                    margin: "0 0 1.25rem 0",
                    font_style: "italic",
                    padding_left: "0.875rem",
                    border_left: "2px solid var(--base02)",
                    "{entry.description}"
                }
                div {
                    display: "flex",
                    flex_wrap: "wrap",
                    gap: "0.5rem",
                    align_items: "center",
                    margin_bottom: "1.75rem",
                    if entry.sponsored {
                        span {
                            font_size: "0.62rem",
                            padding: "0.125rem 0.625rem",
                            background_color: "rgba(198,160,246,0.12)",
                            border: "1px solid var(--base0e)",
                            border_radius: "9999px",
                            color: "var(--base0e)",
                            "Sponsored"
                        }
                    }
                    RatingPill {
                        label: entry.article_type.to_string(),
                        item_key: format!("article_type:{}", entry.article_type),
                        item_ratings,
                    }
                    RatingPill {
                        label: entry.category.to_string(),
                        item_key: format!("category:{}", entry.category),
                        item_ratings,
                    }
                    for source in &article.sources {
                        {
                            let domain = clean_url(&source.url);
                            rsx! {
                                RatingPill {
                                    key: "{source.url}",
                                    label: domain.clone(),
                                    item_key: format!("source:{}", domain),
                                    item_ratings,
                                    url: Some(source.url.clone()),
                                }
                            }
                        }
                    }
                }
                for section in &entry.sections {
                    {render_section(section)}
                }
            } else {
                p {
                    font_size: "0.875rem",
                    color: "var(--base05)",
                    font_style: "italic",
                    "Article content is being generated..."
                }
            }

        }
    }
}
