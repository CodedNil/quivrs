use super::{PageData, decode, jsonld_values};
use itertools::Itertools;
use serde_json::Value;

pub fn parse(page: &PageData) -> Vec<String> {
    if let Some(tags) = page.jsonld.iter().flat_map(jsonld_values).find_map(|obj| {
        obj.get("keywords")
            .map(|kw| match kw {
                Value::Array(arr) => arr
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| decode(s.trim()))
                    .filter(|s| !s.is_empty())
                    .collect(),
                Value::String(s) => split_csv(s),
                _ => Vec::new(),
            })
            .filter(|tags: &Vec<String>| !tags.is_empty())
            .or_else(|| {
                obj.get("articleSection")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| vec![decode(s)])
            })
            .or_else(|| {
                obj.get("content_topic")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(split_csv)
            })
    }) {
        return tags;
    }

    if let Some(tags) = page
        .meta
        .iter()
        .find(|tag| tag.name == "keywords")
        .map(|tag| split_csv(&tag.content))
        .filter(|tags| !tags.is_empty())
    {
        return tags;
    }

    if let Some(tags) = page
        .meta
        .iter()
        .find(|tag| tag.name == "parsely-tags")
        .map(|tag| split_csv(&tag.content))
        .filter(|tags| !tags.is_empty())
    {
        return tags;
    }

    if let Some(tags) = page
        .meta
        .iter()
        .find(|tag| tag.name == "news_keywords")
        .map(|tag| split_csv(&tag.content))
        .filter(|tags| !tags.is_empty())
    {
        return tags;
    }

    let page_sections = page
        .meta
        .iter()
        .filter(|tag| matches!(tag.name.as_str(), "page.section" | "page.subsection"))
        .map(|tag| tag.content.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect_vec();
    if !page_sections.is_empty() {
        return page_sections;
    }

    Vec::new()
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect_vec()
}
