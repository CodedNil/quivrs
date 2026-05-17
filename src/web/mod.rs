mod components;

use crate::shared::{
    ArticleType, Category, Section, StoredArticle, server_functions::get_articles,
};
use components::{base16, image_figure};
use dioxus::prelude::*;
use uuid::Uuid;

pub fn app() -> Element {
    let articles = use_resource(get_articles);
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
                    match &*articles.read_unchecked() {
                        Some(Ok(list)) => rsx! {
                            for article in list {
                                article_item { key: "{article.id}", article: article.clone(), selected }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            p { padding: "1.25rem", font_size: "0.75rem", color: base16::BASE08, "Error: {e}" }
                        },
                        None => rsx! {
                            p { padding: "1.25rem", font_size: "0.75rem", color: base16::BASE03, "Loading..." }
                        },
                    }
                }
            }
            main { flex: "1", overflow_y: "auto",
                {
                    match (*selected.read(), &*articles.read_unchecked()) {
                        (Some(id), Some(Ok(list))) => {
                            list.iter()
                                .find(|a| a.id == id)
                                .map_or_else(|| rsx! {}, |article| rsx! {
                                    article_detail { article: article.clone() }
                                })
                        }
                        _ => rsx! {
                            div {
                                display: "flex",
                                align_items: "center",
                                justify_content: "center",
                                height: "100%",
                                color: base16::BASE05,
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

fn source_parts(s: &str) -> (&str, &str) {
    s.split_once('~').unwrap_or(("", s))
}

fn render_inline(text: &str) -> Element {
    let mut elements: Vec<Element> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut plain_start = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if (b == b'*' || b == b'_')
            && let Some(rel_end) = bytes[i + 1..].iter().position(|&x| x == b)
        {
            let end = rel_end + i + 1;
            if i > plain_start {
                let s = text[plain_start..i].to_string();
                elements.push(rsx! {
                    span { "{s}" }
                });
            }
            let s = text[i + 1..end].to_string();
            if b == b'*' {
                elements.push(rsx! {
                    span { font_weight: "700", "{s}" }
                });
            } else {
                elements.push(rsx! {
                    span { font_style: "italic", "{s}" }
                });
            }
            i = end + 1;
            plain_start = i;
            continue;
        }
        i += 1;
    }

    if plain_start < bytes.len() {
        let s = text[plain_start..].to_string();
        elements.push(rsx! {
            span { "{s}" }
        });
    }

    rsx! {
        {elements.into_iter()}

    }
}

const fn article_type_label(t: &ArticleType) -> &'static str {
    match t {
        ArticleType::BreakingNews => "Breaking News",
        ArticleType::News => "News",
        ArticleType::Opinion => "Opinion",
        ArticleType::Marketing => "Marketing",
        ArticleType::Sale => "Sale",
        ArticleType::Review => "Review",
        ArticleType::Blog => "Blog",
        ArticleType::Newsletter => "Newsletter",
        ArticleType::Video => "Video",
        ArticleType::Post => "Post",
    }
}

const fn category_label(c: &Category) -> &'static str {
    match c {
        Category::World => "World",
        Category::Business => "Business",
        Category::Culture => "Culture",
        Category::Politics => "Politics",
        Category::Health => "Health",
        Category::Technology => "Technology",
        Category::Science => "Science",
        Category::Education => "Education",
        Category::Sports => "Sports",
        Category::Gaming => "Gaming",
    }
}

#[component]
fn article_item(article: StoredArticle, mut selected: Signal<Option<Uuid>>) -> Element {
    let is_selected = *selected.read() == Some(article.id);
    let mut hovered = use_signal(|| false);

    let Some(entry) = &article.entry else {
        return rsx! {};
    };

    let (title, description) = (entry.title.as_str(), entry.description.as_str());

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

    rsx! {
        div {
            padding_top: "0.875rem",
            padding_bottom: "0.875rem",
            padding_right: "1.25rem",
            padding_left: pl,
            cursor: "pointer",
            border_bottom: "1px solid rgba(39, 39, 42, 0.6)",
            border_left: bl,
            background_color: bg,
            onclick: move |_| selected.set(Some(article.id)),
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            h3 {
                font_size: "0.75rem",
                font_weight: "600",
                color: base16::BASE05,
                line_height: "1.375",
                margin: "0 0 0.25rem 0",
                "{title}"
            }
            p {
                font_size: "0.68rem",
                color: base16::BASE05,
                line_height: "1.625",
                margin: "0",
                "{description}"
            }
        }
    }
}

fn render_box_item(item: &str) -> Element {
    rsx! {
        div {
            background_color: base16::BASE01,
            border: "1px solid {base16::BASE02}",
            border_radius: "0.375rem",
            padding: "0.625rem 0.875rem",
            if let Some((header, text)) = item.split_once('~') {
                div {
                    font_size: "0.62rem",
                    font_weight: "700",
                    color: base16::BASE03,
                    text_transform: "uppercase",
                    letter_spacing: "0.08em",
                    margin_bottom: "0.25rem",
                    "{header}"
                }
                div {
                    font_size: "0.875rem",
                    color: base16::BASE05,
                    line_height: "1.5",
                    {render_inline(text)}
                }
            } else {
                div {
                    font_size: "0.875rem",
                    color: base16::BASE05,
                    line_height: "1.5",
                    {render_inline(item)}
                }
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
        Section::Paragraph(text) => {
            rsx! {
                p {
                    font_size: "0.875rem",
                    color: base16::BASE05,
                    line_height: "1.75",
                    margin: "0 0 1rem 0",
                    {render_inline(text)}
                }
            }
        }
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
fn article_detail(article: StoredArticle) -> Element {
    let updated = article.updated_at.format("%Y-%m-%d %H:%M UTC").to_string();
    let Some(entry) = &article.entry else {
        return rsx! {};
    };

    rsx! {
        article {
            max_width: "42rem",
            margin_left: "auto",
            margin_right: "auto",
            padding_left: "2.5rem",
            padding_right: "2.5rem",
            padding_top: "3rem",
            padding_bottom: "5rem",

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
                "{updated}"
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
                margin_bottom: "0.75rem",
                for tag in &entry.tags {
                    span {
                        font_size: "0.62rem",
                        padding: "0.125rem 0.625rem",
                        background_color: base16::BASE02,
                        border: "1px solid {base16::BASE02}",
                        border_radius: "9999px",
                        color: base16::BASE05,
                        "{tag}"
                    }
                }
            }
            div { display: "flex", gap: "0.5rem", margin_bottom: "1.75rem",
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
                span {
                    font_size: "0.62rem",
                    padding: "0.125rem 0.625rem",
                    background_color: "rgba(198,160,246,0.12)",
                    border: "1px solid {base16::BASE0E}",
                    border_radius: "9999px",
                    color: base16::BASE0E,
                    {article_type_label(&entry.article_type)}
                }
                span {
                    font_size: "0.62rem",
                    padding: "0.125rem 0.625rem",
                    background_color: "rgba(166,218,149,0.12)",
                    border: "1px solid {base16::BASE0B}",
                    border_radius: "9999px",
                    color: base16::BASE0B,
                    {category_label(&entry.category)}
                }
            }

            for section in &entry.sections {
                {render_section(section)}
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
                for source in &article.sources {
                    a {
                        display: "block",
                        font_size: "0.75rem",
                        color: base16::BASE05,
                        text_decoration: "none",
                        margin_bottom: "0.375rem",
                        overflow: "hidden",
                        text_overflow: "ellipsis",
                        white_space: "nowrap",
                        href: "{source.url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "{source.title}"
                    }
                }
            }
        }
    }
}
