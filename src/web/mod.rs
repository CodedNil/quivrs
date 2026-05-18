mod components;

use crate::{
    shared::{
        ArticleStatus, Rating, Section, StoredArticle, UserArticle,
        server_functions::{get_all_item_ratings, get_articles, get_user_articles},
    },
    web::components::{
        base16, clean_url, image_figure, rated_tag, rating_button, rating_color, render_box_item,
        render_inline, source_parts, source_pill, status_button,
    },
};
use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub fn app() -> Element {
    let articles = use_resource(get_articles);
    let ua_rev = use_signal(|| 0u32);
    let user_articles = use_resource(move || {
        let _rev = ua_rev();
        get_user_articles()
    });
    let ir_rev = use_signal(|| 0u32);
    let item_ratings = use_resource(move || {
        let _rev = ir_rev();
        get_all_item_ratings()
    });
    let selected = use_signal(|| Option::<Uuid>::None);

    rsx! {
        document::Title { "Quivrs" }
        div {
            display: "flex",
            height: "100vh",
            overflow: "hidden",
            background_color: base16::BASE00,
            color: base16::BASE05,
            aside {
                width: "18rem",
                border_right: "1px solid {base16::BASE02}",
                display: "flex",
                flex_direction: "column",
                background_color: base16::BASE01,
                div {
                    padding: "1rem 1.25rem",
                    border_bottom: "1px solid {base16::BASE02}",
                    flex_shrink: "0",
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
                        match (&*articles.read_unchecked(), &*user_articles.read_unchecked()) {
                            (Some(Ok(arts)), Some(Ok(uas))) => {
                                let art_map =
                                    arts.iter().map(|a| (a.id, a)).collect::<HashMap<_, _>>();
                                let mut new_v = Vec::new();
                                let mut stored_v = Vec::new();
                                let mut binned_v = Vec::new();
                                for ua in uas {
                                    if let Some(&art) = art_map.get(&ua.article_id) {
                                        match ua.status {
                                            ArticleStatus::New => {
                                                new_v.push((art.clone(), ua.clone()));
                                            }
                                            ArticleStatus::Stored => {
                                                stored_v.push((art.clone(), ua.clone()));
                                            }
                                            ArticleStatus::Binned => {
                                                binned_v.push((art.clone(), ua.clone()));
                                            }
                                        }
                                    }
                                }
                                rsx! {
                                    {sidebar_section("New", new_v, selected)}
                                    {sidebar_section("Stored", stored_v, selected)}
                                    {sidebar_section("Binned", binned_v, selected)}
                                }
                            }
                            (Some(Err(e)), _) | (_, Some(Err(e))) => rsx! {
                                p { padding: "1.25rem", font_size: "0.75rem", color: base16::BASE08, "Error: {e}" }
                            },
                            _ => rsx! {
                                p { padding: "1.25rem", font_size: "0.75rem", color: base16::BASE03, "Loading..." }
                            },
                        }
                    }
                }
            }
            main { flex: "1", overflow_y: "auto",
                {
                    let ir_map = match &*item_ratings.read_unchecked() {
                        Some(Ok(m)) => m.clone(),
                        _ => HashMap::new(),
                    };
                    match (
                        *selected.read(),
                        &*articles.read_unchecked(),
                        &*user_articles.read_unchecked(),
                    ) {
                        (Some(id), Some(Ok(arts)), Some(Ok(uas))) => {
                            let art = arts.iter().find(|a| a.id == id);
                            let ua = uas.iter().find(|ua| ua.article_id == id);
                            match (art, ua) {
                                (Some(art), Some(ua)) => rsx! {
                                    article_detail {
                                        article: art.clone(),
                                        user_article: ua.clone(),
                                        ua_rev,
                                        item_ratings: ir_map,
                                        ir_rev,
                                    }
                                },
                                _ => rsx! {},
                            }
                        }
                        _ => rsx! {
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
                    }
                }
            }
        }
    }
}

fn sidebar_section(
    label: &str,
    items: Vec<(StoredArticle, UserArticle)>,
    selected: Signal<Option<Uuid>>,
) -> Element {
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
                "{items.len()}"
            }
        }
        for (art, ua) in items {
            article_item {
                key: "{art.id}",
                article: art,
                user_article: ua,
                selected,
            }
        }
    }
}

#[component]
fn article_item(
    article: StoredArticle,
    user_article: UserArticle,
    mut selected: Signal<Option<Uuid>>,
) -> Element {
    let is_selected = *selected.read() == Some(article.id);
    let mut hovered = use_signal(|| false);

    let Some(entry) = &article.entry else {
        return rsx! {};
    };

    let bg = if is_selected {
        "rgba(23, 37, 84, 0.4)"
    } else if *hovered.read() {
        "rgba(39, 39, 42, 0.4)"
    } else {
        "transparent"
    };
    let bl = if is_selected {
        format!("3px solid {}", base16::BASE0D)
    } else {
        "3px solid transparent".to_string()
    };
    let pl = if is_selected { "17px" } else { "1.25rem" };
    let dot_color = user_article.rating.map_or("transparent", rating_color);

    rsx! {
        div {
            padding_top: "0.75rem",
            padding_bottom: "0.75rem",
            padding_right: "1.25rem",
            padding_left: pl,
            cursor: "pointer",
            border_bottom: "1px solid rgba(39, 39, 42, 0.6)",
            border_left: bl,
            background_color: bg,
            onclick: move |_| selected.set(Some(article.id)),
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
                    opacity: if user_article.rating.is_some() { "1" } else { "0" },
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

fn render_section(section: &Section) -> Element {
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
                image_figure {
                    url: url.to_string(),
                    caption: caption.to_string(),
                    margin: "1.25rem 0",
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
                        {
                            if let Some((header, text)) = item.split_once('~') {
                                let header = header.to_string();
                                rsx! {
                                    span { font_weight: "700", "{header}" }
                                    " "
                                    {render_inline(text)}
                                }
                            } else {
                                render_inline(item)
                            }
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

#[component]
fn article_detail(
    article: StoredArticle,
    user_article: UserArticle,
    ua_rev: Signal<u32>,
    item_ratings: HashMap<String, Rating>,
    ir_rev: Signal<u32>,
) -> Element {
    let id = article.id;
    let status = user_article.status;
    let rating = user_article.rating;

    rsx! {
        article {
            max_width: "42rem",
            margin_left: "auto",
            margin_right: "auto",
            padding_left: "2.5rem",
            padding_right: "2.5rem",
            padding_top: "2rem",
            padding_bottom: "5rem",

            div {
                display: "flex",
                align_items: "center",
                justify_content: "space-between",
                padding_bottom: "1rem",
                margin_bottom: "1.5rem",
                border_bottom: "1px solid {base16::BASE02}",
                div { display: "flex", gap: "0.375rem",
                    {status_button("New", ArticleStatus::New, status, id, ua_rev)}
                    {status_button("Store", ArticleStatus::Stored, status, id, ua_rev)}
                    {status_button("Bin", ArticleStatus::Binned, status, id, ua_rev)}
                }
                div { display: "flex", gap: "0.25rem",
                    {rating_button("Hate", Rating::Hated, rating, id, ua_rev)}
                    {rating_button("Dislike", Rating::Disliked, rating, id, ua_rev)}
                    {rating_button("Neutral", Rating::Neutral, rating, id, ua_rev)}
                    {rating_button("Like", Rating::Liked, rating, id, ua_rev)}
                    {rating_button("Love", Rating::Loved, rating, id, ua_rev)}
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
                    gap: "0.375rem",
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
                    rated_tag {
                        label: entry.article_type.to_string(),
                        item_key: format!("article_type:{}", entry.article_type),
                        rating: item_ratings.get(&format!("article_type:{}", entry.article_type)).copied(),
                        tag_color: base16::BASE0E,
                        ir_rev,
                    }
                    rated_tag {
                        label: entry.category.to_string(),
                        item_key: format!("category:{}", entry.category),
                        rating: item_ratings.get(&format!("category:{}", entry.category)).copied(),
                        tag_color: base16::BASE0B,
                        ir_rev,
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

            div {
                padding_top: "1.25rem",
                border_top: "1px solid {base16::BASE02}",
                h4 {
                    font_size: "0.62rem",
                    color: base16::BASE03,
                    text_transform: "uppercase",
                    letter_spacing: "0.1em",
                    margin: "0 0 0.625rem 0",
                    "Original Sources"
                }
                div { display: "flex", flex_wrap: "wrap", gap: "0.5rem",
                    for source in &article.sources {
                        source_pill {
                            url: source.url.clone(),
                            rating: item_ratings
                                                                                        .get(&format!("source:{}", clean_url(&source.url)))
                                                                                        .copied(),
                            ir_rev,
                        }
                    }
                }
            }
        }
    }
}
