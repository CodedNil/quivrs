use crate::shared::{
    ArticleData, ArticleStatus, Category, Rating, Section, StoredArticle,
    server_functions::{
        get_all_item_ratings, get_user_articles, reclassify_articles, set_article_status,
        set_item_rating, set_rating,
    },
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape, icons::fa_solid_icons};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

mod style {
    pub const RADIUS_CARD: &str = "0.625rem";
    pub const RADIUS_GROUP: &str = "0.875rem";

    pub const GROUP_SHADOW: &str = "inset 0 2px 10px rgba(0,0,0,0.4), 0 1px 4px rgba(0,0,0,0.22)";

    pub const CARD_BG_IDLE: &str = "var(--base00)";
    pub const CARD_BG_SELECTED: &str = "color-mix(in srgb, var(--base0d) 22%, var(--base00))";

    pub const CARD_SHADOW_IDLE: &str = "0 1px 3px rgba(0,0,0,0.22)";
    pub const CARD_SHADOW_HOVER: &str = "0 4px 14px rgba(0,0,0,0.3)";
    pub const CARD_SHADOW_SELECTED: &str =
        "0 4px 18px rgba(0,0,0,0.38), 0 2px 8px color-mix(in srgb, var(--base0d) 30%, transparent)";
    pub const CARD_SHADOW_ACTIVE: &str = "0 1px 2px rgba(0,0,0,0.18)";

    pub const TRANSITION_CARD: &str =
        "transform 0.12s ease, box-shadow 0.18s ease, background-color 0.15s ease";
}

const fn rating_color(r: Rating) -> &'static str {
    match r {
        Rating::Hated => "#873535",
        Rating::Disliked => "#A3674E",
        Rating::Neutral => "#494d64",
        Rating::Liked => "#5D7D31",
        Rating::Loved => "#3E6E2F",
    }
}

#[component]
fn AppHead() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap",
        }
        // <https://raw.githubusercontent.com/chriskempson/base16/refs/heads/main/styling.md>
        // Colours base00 to base07 are typically variations of a shade and run from darkest to lightest. These colours are used for foreground and background, status bars, line highlighting and such. colours base08 to base0F are typically individual colours used for types, operators, names and variables. In order to create a dark theme, colours base00 to base07 should span from dark to light. For a light theme, these colours should span from light to dark.
        document::Style {
            "body {{
                --base00: #24273a; --base01: #1e2030; --base02: #363a4f; --base03: #494d64;
                --base04: #5b6078; --base05: #cad3f5; --base06: #f4dbd6; --base07: #b7bdf8;
                --base08: #ed8796; --base09: #f5a97f; --base0a: #eed49f; --base0b: #a6da95;
                --base0c: #8bd5ca; --base0d: #8aadf4; --base0e: #c6a0f6; --base0f: #f0c6c6;
                font-family: 'Inter', system-ui, sans-serif;
                margin: 0; padding: 0;
            }}
            ::selection {{
                background-color: color-mix(in srgb, var(--base0d) 40%, transparent);
                color: var(--base05);
            }}"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[layout(MainLayout)]
    #[route("/")]
    Home {},
    #[route("/entry/:id")]
    Article { id: Uuid },
}

#[component]
fn Home() -> Element {
    rsx! {
        div {
            display: "flex",
            align_items: "center",
            justify_content: "center",
            height: "100%",
            color: "var(--base04)",
            font_size: "0.875rem",
            "Select an article to read"
        }
    }
}

#[component]
fn Article(id: Uuid) -> Element {
    let articles = use_context::<Signal<Vec<ArticleData>>>();
    let item_ratings = use_context::<Signal<HashMap<String, Rating>>>();

    let found = articles
        .read()
        .iter()
        .find(|a| a.id == id)
        .map(|a| (a.status, a.rating, a.article.clone()));

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
        None => rsx! {
            div {
                display: "flex",
                align_items: "center",
                justify_content: "center",
                height: "100%",
                color: "var(--base04)",
                font_size: "0.875rem",
                "Article not found"
            }
        },
    }
}

pub fn app() -> Element {
    let mut articles: Signal<Vec<ArticleData>> = use_signal(Vec::new);
    let mut item_ratings: Signal<HashMap<String, Rating>> = use_signal(HashMap::new);

    use_context_provider(|| articles);
    use_context_provider(|| item_ratings);

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
        Router::<Route> {}
    }
}
#[component]
fn MainLayout() -> Element {
    let mut articles: Signal<Vec<ArticleData>> = use_context();
    let _item_ratings: Signal<HashMap<String, Rating>> = use_context();
    let route = use_route::<Route>();

    let selected_id = match route {
        Route::Article { id } => Some(id),
        Route::Home {} => None,
    };

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Some(target_id) = selected_id {
            // Scroll to the selected article in the sidebar
            spawn(async move {
                use wasm_bindgen::JsCast;
                let window = web_sys::window().unwrap();
                let _ = gloo_timers::future::TimeoutFuture::new(100).await;
                let document = window.document().unwrap();
                if let Some(element) = document.get_element_by_id(&format!("article-{}", target_id))
                {
                    if let Ok(html_element) = element.dyn_into::<web_sys::HtmlElement>() {
                        html_element.scroll_into_view();
                    }
                }
            });
        }
    });

    rsx! {
        AppHead {}
        document::Title { "Quivrs" }
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
                background_color: "var(--base0d)",

                div {
                    padding: "1rem 1.25rem",
                    border_bottom: "1px solid var(--base02)",
                    display: "flex",
                    justify_content: "space-between",
                    align_items: "center",
                    h1 {
                        font_size: "0.75rem",
                        font_weight: "700",
                        letter_spacing: "0.1em",
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
                            for a in data.iter() {
                                match a.status {
                                    ArticleStatus::New => new_v.push((a.id, a.rating, &a.article)),
                                    ArticleStatus::Stored => stored_v.push((a.id, a.rating, &a.article)),
                                    ArticleStatus::Binned => binned_v.push((a.id, a.rating, &a.article)),
                                }
                            }
                            rsx! {
                                {sidebar_section("New", new_v, selected_id, true)}
                                {sidebar_section("Stored", stored_v, selected_id, false)}
                                {sidebar_section("Binned", binned_v, selected_id, false)}
                            }
                        }
                    }
                }
            }

            main { flex: "1", overflow_y: "auto", Outlet::<Route> {} }
        }
    }
}

fn sidebar_section(
    label: &str,
    items: Vec<(Uuid, Option<Rating>, &StoredArticle)>,
    selected: Option<Uuid>,
    group_by_category: bool,
) -> Element {
    let total_items = items.len();

    let content = if group_by_category {
        let mut groups: BTreeMap<Category, Vec<_>> = BTreeMap::new();
        for (id, rating, article) in items {
            groups
                .entry(article.category)
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
            for (id, rating, article) in items {
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
    selected: Option<Uuid>,
) -> Element {
    let is_selected = selected == Some(id);
    let mut hovered = use_signal(|| false);
    let mut pressed = use_signal(|| false);

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
            id: "article-{id}",
            cursor: "pointer",
            position: "relative",
            border_radius: style::RADIUS_CARD,
            margin: "0.375rem 0.5rem",
            overflow: "hidden",
            background_color: bg,
            box_shadow: shadow,
            transform: scale,
            transition: style::TRANSITION_CARD,
            onmouseleave: move |_| {
                hovered.set(false);
                pressed.set(false);
            },
            onmousedown: move |_| pressed.set(true),
            onmouseup: move |_| pressed.set(false),
            onclick: move |_| {
                use_navigator().push(Route::Article { id });
            },
            onmouseenter: move |_| hovered.set(true),

            if let Some(img_url) = hero_image {
                img {
                    src: "{img_url}",
                    alt: "",
                    width: "100%",
                    height: "144px",
                    object_fit: "cover",
                    display: "block",
                }
            }

            // Rating notch: triangle clipped to top-left corner.
            // The filter wrapper is needed because drop-shadow on a clip-path
            // element is itself clipped; the parent applies it to the painted shape.
            if let Some(r) = rating {
                div {
                    position: "absolute",
                    top: "0",
                    left: "0",
                    width: "48px",
                    height: "48px",
                    z_index: 1,
                    filter: "drop-shadow(1px 2px 5px rgba(0,0,0,0.75))",
                    div {
                        width: "100%",
                        height: "100%",
                        background_color: rating_color(r),
                        clip_path: "polygon(0 0, 100% 0, 0 100%)",
                    }
                }
            }

            div { padding: "0.625rem 0.75rem",
                h3 {
                    font_size: "0.75rem",
                    font_weight: "600",
                    color: "var(--base05)",
                    line_height: "1.375",
                    margin: "0 0 0.2rem 0",
                    "{title}"
                }
                p {
                    font_size: "0.67rem",
                    color: "var(--base05)",
                    line_height: "1.5",
                    margin: "0",
                    "{description}"
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

    let first_source = article.sources.first();
    let title = article.display_title();
    let description = article.display_description();

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
                    RefreshButton {
                        title: "Refresh classification",
                        onclick: move |_| async move {
                            if reclassify_articles(vec![id]).await.is_ok()
                                && let Ok(new_articles) = get_user_articles().await
                            {
                                articles.set(new_articles);
                            }
                        },
                    }
                    {rating_button("Hate", Rating::Hated, rating, id, articles)}
                    {rating_button("Dislike", Rating::Disliked, rating, id, articles)}
                    {rating_button("Neutral", Rating::Neutral, rating, id, articles)}
                    {rating_button("Like", Rating::Liked, rating, id, articles)}
                    {rating_button("Love", Rating::Loved, rating, id, articles)}
                }
            }

            h1 {
                font_size: "1.5rem",
                font_weight: "700",
                line_height: "1.25",
                color: "var(--base05)",
                margin: "0 0 0.375rem 0",
                "{title}"
            }
            p {
                font_size: "0.7rem",
                color: "var(--base05)",
                margin: "0 0 1rem 0",
                {article.published.format("%Y-%m-%d %H:%M UTC").to_string()}
            }
            p {
                font_size: "0.875rem",
                color: "var(--base05)",
                line_height: "1.625",
                margin: "0 0 1.25rem 0",
                font_style: "italic",
                padding_left: "0.875rem",
                border_left: "2px solid var(--base02)",
                "{description}"
            }
            div {
                display: "flex",
                flex_wrap: "wrap",
                gap: "0.5rem",
                align_items: "center",
                margin_bottom: "1.75rem",
                RatingPill {
                    label: article.category.to_string(),
                    item_key: format!("category:{}", article.category),
                    item_ratings,
                }
                for source in &article.sources {
                    RatingPill {
                        key: "{source.url}",
                        label: source.source.clone(),
                        item_key: format!("source:{}", source.source),
                        item_ratings,
                        url: Some(source.url.clone()),
                    }
                }
            }
            if let Some(entry) = &article.entry {
                for section in &entry.sections {
                    {render_section(section)}
                }
            } else {
                "Generating..."
                div { {first_source.map_or("", |s| &s.content)} }
            }
        }
    }
}

fn source_parts(s: &str) -> (&str, &str) {
    s.split_once('|').unwrap_or(("", s))
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

fn render_box_item(item: &str) -> Element {
    rsx! {
        div {
            background_color: "var(--base01)",
            border: "1px solid var(--base02)",
            border_radius: "0.375rem",
            padding: "0.625rem 0.875rem",
            height: "100%",
            if let Some((header, text)) = item.split_once('|') {
                div {
                    font_size: "0.62rem",
                    font_weight: "700",
                    color: "var(--base03)",
                    text_transform: "uppercase",
                    letter_spacing: "0.08em",
                    margin_bottom: "0.25rem",
                    "{header}"
                }
                div {
                    font_size: "0.875rem",
                    color: "var(--base05)",
                    line_height: "1.5",
                    {render_inline(text)}
                }
            } else {
                div {
                    font_size: "0.875rem",
                    color: "var(--base05)",
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
                font_size: "0.8rem",
                font_weight: "700",
                color: "var(--base05)",
                text_transform: "uppercase",
                letter_spacing: "0.1em",
                margin: "2.5rem 0 0.8rem 0",
                "{header}"
            }
        },
        Section::Paragraph(text) => rsx! {
            p {
                font_size: "0.875rem",
                color: "var(--base05)",
                line_height: "1.75",
                margin: "0 0 1rem 0",
                {render_inline(text)}
            }
        },
        Section::Image(raw) => {
            let (url, caption) = source_parts(raw);
            rsx! {
                figure { margin: "1.25rem 0",
                    img {
                        src: "{url}",
                        alt: "{caption}",
                        width: "100%",
                        border_radius: "0.375rem",
                        display: "block",
                    }
                    figcaption {
                        font_size: "0.7rem",
                        color: "var(--base03)",
                        margin_top: "0.375rem",
                        text_align: "center",
                        "{caption}"
                    }
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
                        color: "var(--base05)",
                        line_height: "1.6",
                        if let Some((header, text)) = item.split_once('|') {
                            span { font_weight: "700", "{header}" }
                            " "
                            {render_inline(text)}
                        } else {
                            {render_inline(item)}
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
fn RefreshButton(title: String, onclick: EventHandler<MouseEvent>) -> Element {
    let mut hovered = use_signal(|| false);
    rsx! {
        button {
            font_size: "0.875rem",
            line_height: "1",
            padding: "0.2rem 0.5rem",
            border_radius: "9999px",
            background_color: "var(--base02)",
            color: if hovered() { "var(--base06)" } else { "var(--base05)" },
            border: "none",
            cursor: "pointer",
            transition: "color 0.15s ease, border-color 0.15s ease",
            title,
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            onclick,
            "↻"
        }
    }
}

// Shared pill-shaped button used for all status/rating controls.
#[component]
fn PillButton(
    label: String,
    active: bool,
    color: &'static str,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let border_color = if active { color } else { "var(--base02)" };
    rsx! {
        button {
            font_size: "0.62rem",
            font_weight: if active { "600" } else { "400" },
            padding: "0.2rem 0.625rem",
            border_radius: "9999px",
            background_color: if active { color } else { "transparent" },
            color: if active { "var(--base01)" } else { "var(--base04)" },
            border: "1px solid {border_color}",
            cursor: "pointer",
            onclick,
            "{label}"
        }
    }
}

// A pill that shows a label and inline rating buttons
#[component]
fn RatingPill(
    label: String,
    item_key: String,
    item_ratings: Signal<HashMap<String, Rating>>,
    url: Option<String>,
) -> Element {
    let current = item_ratings.read().get(&item_key).copied();
    let bg_color = rating_color(current.unwrap_or(Rating::Neutral));
    let mut is_hovered = use_signal(|| false);

    rsx! {
        div {
            display: "inline-flex",
            align_items: "center",
            background_color: bg_color,
            border_radius: "9999px",
            padding: "0.2rem 0.4rem",
            overflow: "hidden",
            color: "var(--base05)",
            font_size: "0.62rem",
            font_weight: "700",
            transition: "all 0.2s ease-in-out",
            onmouseenter: move |_| is_hovered.set(true),
            onmouseleave: move |_| is_hovered.set(false),

            div {
                display: "flex",
                align_items: "center",
                max_width: if is_hovered() { "100px" } else { "0" },
                opacity: if is_hovered() { "1" } else { "0" },
                transition: "all 0.2s ease-in-out",
                overflow: "hidden",
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

            div { padding: "0.125rem 0.625rem",
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

            div {
                display: "flex",
                align_items: "center",
                max_width: if is_hovered() { "100px" } else { "0" },
                opacity: if is_hovered() { "1" } else { "0" },
                transition: "all 0.2s ease-in-out",
                overflow: "hidden",
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
fn RatingPillBtn<T: IconShape + Clone + PartialEq + 'static>(
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

fn status_button(
    label: &str,
    this_status: ArticleStatus,
    current: ArticleStatus,
    id: Uuid,
    mut articles: Signal<Vec<ArticleData>>,
) -> Element {
    rsx! {
        PillButton {
            label: label.to_string(),
            active: current == this_status,
            color: "var(--base07)",
            onclick: move |_| async move {
                let _ = set_article_status(id, this_status).await;
                if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                    a.status = this_status;
                }
            },
        }
    }
}

fn rating_button(
    label: &str,
    this_rating: Rating,
    current: Option<Rating>,
    id: Uuid,
    mut articles: Signal<Vec<ArticleData>>,
) -> Element {
    rsx! {
        PillButton {
            label: label.to_string(),
            active: current == Some(this_rating),
            color: rating_color(this_rating),
            onclick: move |_| async move {
                let _ = set_rating(id, this_rating).await;
                if let Some(a) = articles.write().iter_mut().find(|a| a.id == id) {
                    a.rating = Some(this_rating);
                }
            },
        }
    }
}

#[component]
fn SectionHeader(label: String, count: usize) -> Element {
    rsx! {
        div {
            padding: "0.75rem 1.25rem 0.5rem",
            display: "flex",
            justify_content: "space-between",
            align_items: "center",
            span {
                font_size: "0.62rem",
                font_weight: "700",
                letter_spacing: "0.1em",
                text_transform: "uppercase",
                color: "var(--base04)",
                "{label}"
            }
            span {
                font_size: "0.62rem",
                color: "var(--base04)",
                background_color: "var(--base02)",
                padding: "0.1rem 0.45rem",
                border_radius: "9999px",
                "{count}"
            }
        }
    }
}

#[component]
fn CategoryGroup(category: Category, children: Element) -> Element {
    fn render_icon<T: dioxus_free_icons::IconShape + Copy + PartialEq + 'static>(
        icon: T,
    ) -> Element {
        rsx! {
            Icon {
                width: 24,
                height: 24,
                fill: "var(--base05)",
                icon,
            }
        }
    }

    rsx! {
        div {
            display: "flex",
            margin: "0 0.625rem 1rem",
            border_radius: style::RADIUS_GROUP,
            clip_path: "inset(0 round 0.875rem)",
            background_color: "var(--base01)",
            box_shadow: style::GROUP_SHADOW,

            div {
                width: "44px",
                min_width: "44px",
                background_color: "var(--base02)",
                div {
                    position: "sticky",
                    top: "0",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    padding: "0.875rem 0.5rem 0.875rem",
                    gap: "0.75rem",
                    {
                        match category {
                            Category::Business => render_icon(fa_solid_icons::FaBriefcase),
                            Category::Politics => render_icon(fa_solid_icons::FaLandmark),
                            Category::Law => render_icon(fa_solid_icons::FaGavel),
                            Category::Health => render_icon(fa_solid_icons::FaHeart),
                            Category::Culture => render_icon(fa_solid_icons::FaFilm),
                            Category::Lifestyle => render_icon(fa_solid_icons::FaHouse),
                            Category::Transport => render_icon(fa_solid_icons::FaCar),
                            Category::Nature => render_icon(fa_solid_icons::FaLeaf),
                            Category::Technology => render_icon(fa_solid_icons::FaMicrochip),
                            Category::Software => render_icon(fa_solid_icons::FaCode),
                            Category::AI => render_icon(fa_solid_icons::FaBrain),
                            Category::Science => render_icon(fa_solid_icons::FaFlask),
                            Category::Sports => render_icon(fa_solid_icons::FaFootball),
                            Category::Gaming => render_icon(fa_solid_icons::FaGamepad),
                        }
                    }
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
