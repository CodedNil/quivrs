mod components;

use crate::{
    shared::{
        ArticleData, ArticleEntry, ArticleStatus, Category, Rating, StoredArticle,
        server_functions::{get_all_item_ratings, get_user_articles},
    },
    web::components::{
        CategoryIcon, RatingPill, base16, clean_url, rating_button, rating_color, render_section,
        status_button,
    },
};
use dioxus::{html::base, prelude::*};
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
        document::Style { "body {{ margin: 0; padding: 0; }}" }
        div {
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            background_color: base16::BASE00,
            color: base16::BASE05,

            aside {
                width: "27rem",
                border_right: "1px solid {base16::BASE02}",
                display: "flex",
                flex_direction: "column",
                background_color: base16::BASE01,

                div {
                    padding: "1rem 1.25rem",
                    border_bottom: "1px solid {base16::BASE02}",
                    h1 {
                        font_size: "0.75rem",
                        font_weight: "700",
                        letter_spacing: "0.1em",
                        color: base16::BASE05,
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
                                p { padding: "1.25rem", font_size: "0.75rem", color: base16::BASE03, "Loading..." }
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
                                    color: base16::BASE04,
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

    // Prepare and render the item content based on grouping rules
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
                div { display: "flex", margin_bottom: "1.5rem", background_color: base16::BASE01,

                    div {
                        width: "24px",
                        min_width: "24px",
                        background_color: base16::BASE02,
                        border_radius: "0.25rem 0 0 0.25rem",

                        div {
                            position: "sticky",
                            top: "0",
                            display: "flex",
                            flex_direction: "column",
                            align_items: "center",
                            padding_top: "0.75rem",
                            gap: "0.75rem",

                            CategoryIcon { category }
                            div {
                                style: "writing-mode: vertical-rl; transform: rotate(180deg);",
                                font_size: "0.5rem",
                                font_weight: "800",
                                color: base16::BASE03,
                                text_transform: "uppercase",
                                letter_spacing: "0.1em",
                                "{category}"
                            }
                        }
                    }
                    div { flex: "1",
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

    // Render the final layout
    rsx! {
        div {
            padding: "0.75rem 1.25rem 0.25rem",
            display: "flex",
            justify_content: "space-between",
            align_items: "center",
            span {
                font_size: "0.62rem",
                font_weight: "700",
                letter_spacing: "0.1em",
                text_transform: "uppercase",
                color: base16::BASE03,
                "{label}"
            }
            span {
                font_size: "0.62rem",
                color: base16::BASE03,
                background_color: base16::BASE02,
                padding: "0.1rem 0.4rem",
                border_radius: "9999px",
                "{total_items}"
            }
        }
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

    let Some(entry) = &article.entry else {
        return rsx! {};
    };

    let dot_color = rating.map_or("transparent", rating_color);

    rsx! {
        div {
            padding: "0.75rem 1.25rem",
            padding_left: if is_selected { "17px" } else { "1.25rem" },
            cursor: "pointer",
            border_bottom: "1px solid rgba(39, 39, 42, 0.6)",
            border_left: if is_selected { "3px solid {base16::BASE0D}" } else { "3px solid transparent" },
            background_color: if is_selected { "rgba(23, 37, 84, 0.4)" } else if *hovered.read() { "rgba(39, 39, 42, 0.4)" } else { "transparent" },
            onclick: move |_| selected.set(Some(id)),
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            div {
                display: "flex",
                align_items: "center",
                gap: "0.4rem",
                margin_bottom: "0.25rem",
                div {
                    width: "7px",
                    height: "7px",
                    min_width: "7px",
                    border_radius: "50%",
                    background_color: dot_color,
                    opacity: if rating.is_some() { "1" } else { "0" },
                }
                h3 {
                    font_size: "0.75rem",
                    font_weight: "600",
                    color: base16::BASE05,
                    line_height: "1.375",
                    margin: "0",
                    "{entry.title}"
                }
            }
            p {
                font_size: "0.68rem",
                color: base16::BASE05,
                line_height: "1.625",
                margin: "0",
                padding_left: "1.1rem",
                "{entry.description}"
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
                border_bottom: "1px solid {base16::BASE02}",
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
                    color: base16::BASE05,
                    margin: "0 0 0.375rem 0",
                    "{entry.title}"
                }
                p {
                    font_size: "0.7rem",
                    color: base16::BASE03,
                    margin: "0 0 1rem 0",
                    {article.updated_at.format("%Y-%m-%d %H:%M UTC").to_string()}
                }
                p {
                    font_size: "0.875rem",
                    color: base16::BASE05,
                    line_height: "1.625",
                    margin: "0 0 1.25rem 0",
                    font_style: "italic",
                    padding_left: "0.875rem",
                    border_left: "2px solid {base16::BASE02}",
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
                            border: "1px solid {base16::BASE0E}",
                            border_radius: "9999px",
                            color: base16::BASE0E,
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
                    color: base16::BASE04,
                    font_style: "italic",
                    "Article content is being generated..."
                }
            }

        }
    }
}
