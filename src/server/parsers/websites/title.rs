use super::{PageData, decode, jsonld_values};
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;

const MIN_TITLE_LEN: usize = 6;

static SEL_TITLE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("title").unwrap());

pub fn parse(page: &PageData) -> Option<String> {
    if let Some(title) = page.jsonld.iter().flat_map(jsonld_values).find_map(|obj| {
        obj.get("headline")
            .or_else(|| obj.get("name"))
            .and_then(Value::as_str)
            .map(decode)
            .filter(|s| s.len() > MIN_TITLE_LEN)
    }) {
        return Some(title);
    }

    if let Some(title) = page
        .meta
        .iter()
        .find(|tag| matches!(tag.name.as_str(), "og:title" | "twitter:title"))
        .map(|tag| decode(&tag.content))
        .filter(|s| s.len() > MIN_TITLE_LEN)
    {
        return Some(title);
    }

    let doc = Html::parse_document(&page.html);
    doc.select(&SEL_TITLE)
        .next()
        .map(|el| el.text().collect::<String>())
        .map(|raw| {
            raw.rfind(" | ")
                .or_else(|| raw.rfind(" \u{2014} "))
                .or_else(|| raw.rfind(" - "))
                .map_or(raw.as_str(), |pos| &raw[..pos])
                .trim()
                .to_string()
        })
        .map(|s| decode(&s))
        .filter(|s| s.len() > MIN_TITLE_LEN)
}
