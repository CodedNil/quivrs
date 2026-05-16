mod components;

use crate::shared::{Section, StoredArticle, server_functions::get_articles};
use components::{base16, image_figure, labeled_row, section_heading, surface_card};
use dioxus::prelude::*;

pub fn app() -> Element {
    let articles = use_resource(get_articles);
    let selected = use_signal(|| Option::<StoredArticle>::None);

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
                    selected
                        .read()
                        .as_ref()
                        .map_or_else(|| rsx! {
                            div {
                                display: "flex",
                                align_items: "center",
                                justify_content: "center",
                                height: "100%",
                                color: base16::BASE05,
                                font_size: "0.875rem",
                                "Select an article to read"
                            }
                        }, |article| rsx! {
                            article_detail { article: article.clone() }
                        })
                }
            }
        }
    }
}

fn article_title(article: &StoredArticle) -> &str {
    if let Some(entry) = &article.entry {
        &entry.title
    } else {
        article.sources.first().map_or("Untitled", |s| &s.title)
    }
}

fn source_parts(s: &str) -> (&str, &str) {
    s.split_once('~').unwrap_or(("", s))
}

#[component]
fn article_item(article: StoredArticle, mut selected: Signal<Option<StoredArticle>>) -> Element {
    let Some(entry) = &article.entry else {
        return rsx! {};
    };
    let is_selected = selected.read().as_ref().map(|a| a.id) == Some(article.id);
    let mut hovered = use_signal(|| false);

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
            onclick: move |_| selected.set(Some(article.clone())),
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            h3 {
                font_size: "0.75rem",
                font_weight: "600",
                color: base16::BASE05,
                line_height: "1.375",
                margin: "0 0 0.25rem 0",
                "{article_title(&article)}"
            }
            p {
                font_size: "0.68rem",
                color: base16::BASE05,
                line_height: "1.625",
                margin: "0",
                "{entry.description}"
            }
        }
    }
}

fn render_section(section: &Section) -> Element {
    match section {
        Section::Text(text) => rsx! {
            p {
                font_size: "0.875rem",
                color: base16::BASE05,
                line_height: "1.75",
                margin: "0 0 1rem 0",
                "{text}"
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
        Section::Highlights(highlights) => rsx! {
            div { margin_bottom: "1.75rem",
                section_heading { label: "Highlights" }
                div {
                    display: "grid",
                    grid_template_columns: "repeat(auto-fill, minmax(14rem, 1fr))",
                    gap: "0.625rem",
                    for h in highlights {
                        {
                            let (header, body) = source_parts(h);
                            rsx! {
                                surface_card {
                                    p {
                                        font_size: "0.7rem",
                                        font_weight: "600",
                                        color: base16::BASE05,
                                        margin: "0 0 0.25rem 0",
                                        "{header}"
                                    }
                                    p {
                                        font_size: "0.8rem",
                                        color: base16::BASE05,
                                        line_height: "1.5",
                                        margin: "0",
                                        "{body}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        Section::Perspectives(perspectives) => rsx! {
            div { margin_bottom: "1.75rem",
                section_heading { label: "Perspectives" }
                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "0.5rem",
                    for pv in perspectives {
                        {
                            let (who, body) = source_parts(pv);
                            rsx! {
                                surface_card {
                                    labeled_row {
                                        label: who.to_string(),
                                        body: body.to_string(),
                                        label_color: base16::BASE0E.to_string(),
                                        label_width: "7rem",
                                        gap: "0.75rem",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        Section::Background(bg) => rsx! {
            div { margin_bottom: "1.75rem",
                section_heading { label: "Background" }
                surface_card { padding: "0.875rem".to_string(),
                    p {
                        font_size: "0.8rem",
                        color: base16::BASE05,
                        line_height: "1.625",
                        margin: "0 0 0.875rem 0",
                        "{bg.text}"
                    }
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "0.5rem",
                        for event in &bg.timeline {
                            {
                                let (date, text) = source_parts(event);
                                rsx! {
                                    labeled_row {
                                        label: date.to_string(),
                                        body: text.to_string(),
                                        label_color: base16::BASE04.to_string(),
                                        label_width: "5.5rem",
                                        gap: "0.875rem",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        Section::TwoImages(raw) => {
            let (left, right) = raw.split_once('|').unwrap_or((raw, ""));
            let (left_url, left_cap) = source_parts(left);
            let (right_url, right_cap) = source_parts(right);
            rsx! {
                div {
                    display: "grid",
                    grid_template_columns: "1fr 1fr",
                    gap: "0.625rem",
                    margin: "1.25rem 0",
                    image_figure {
                        url: left_url.to_string(),
                        caption: left_cap.to_string(),
                        margin: "0",
                    }
                    image_figure {
                        url: right_url.to_string(),
                        caption: right_cap.to_string(),
                        margin: "0",
                    }
                }
            }
        }
        Section::ProsCons(pros_cons) => rsx! {
            div { margin_bottom: "1.75rem",
                div {
                    display: "grid",
                    grid_template_columns: "1fr 1fr",
                    gap: "0.75rem",
                    surface_card {
                        p {
                            font_size: "0.62rem",
                            font_weight: "700",
                            color: base16::BASE05,
                            text_transform: "uppercase",
                            letter_spacing: "0.1em",
                            margin: "0 0 0.625rem 0",
                            "The Good"
                        }
                        ul {
                            margin: "0",
                            padding_left: "1rem",
                            display: "flex",
                            flex_direction: "column",
                            gap: "0.375rem",
                            for part in pros_cons.pros.split(',') {
                                li {
                                    font_size: "0.75rem",
                                    color: base16::BASE05,
                                    line_height: "1.4",
                                    "{part.trim()}"
                                }
                            }
                        }
                    }
                    surface_card {
                        p {
                            font_size: "0.62rem",
                            font_weight: "700",
                            color: base16::BASE05,
                            text_transform: "uppercase",
                            letter_spacing: "0.1em",
                            margin: "0 0 0.625rem 0",
                            "The Bad"
                        }
                        ul {
                            margin: "0",
                            padding_left: "1rem",
                            display: "flex",
                            flex_direction: "column",
                            gap: "0.375rem",
                            for part in pros_cons.cons.split(',') {
                                li {
                                    font_size: "0.75rem",
                                    color: base16::BASE05,
                                    line_height: "1.4",
                                    "{part.trim()}"
                                }
                            }
                        }
                    }
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
                "{article_title(&article)}"
            }
            p {
                font_size: "0.7rem",
                color: base16::BASE05,
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
                margin_bottom: "1.75rem",
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

            for section in &entry.sections {
                {render_section(section)}
            }

            div { margin_bottom: "1.75rem",
                section_heading { label: "References" }
                ol {
                    margin: "0",
                    padding_left: "1.25rem",
                    display: "flex",
                    flex_direction: "column",
                    gap: "0.25rem",
                    for source in &entry.sources {
                        {
                            let (url, title) = source_parts(source);
                            rsx! {
                                li { font_size: "0.75rem", color: base16::BASE05,
                                    a {
                                        color: base16::BASE05,
                                        text_decoration: "none",
                                        href: "{url}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        "{title}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div {
                padding_top: "1.25rem",
                border_top: "1px solid {base16::BASE02}",
                h4 {
                    font_size: "0.62rem",
                    color: base16::BASE05,
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
                        margin_bottom: "0.375rem",
                        text_overflow: "ellipsis",
                        overflow: "hidden",
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
