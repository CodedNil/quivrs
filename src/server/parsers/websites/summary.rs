use super::{PageData, decode, jsonld_values};
use serde_json::Value;

const MIN_SUMMARY_LEN: usize = 26;

pub fn parse(page: &PageData) -> Option<String> {
    if let Some(summary) = page.jsonld.iter().flat_map(jsonld_values).find_map(|obj| {
        obj.get("description")
            .or_else(|| obj.get("alternativeHeadline"))
            .and_then(Value::as_str)
            .map(decode)
            .filter(|s| s.len() > MIN_SUMMARY_LEN)
    }) {
        return Some(summary);
    }

    if let Some(summary) = page
        .meta
        .iter()
        .find(|tag| matches!(tag.name.as_str(), "og:description" | "twitter:description"))
        .map(|tag| decode(&tag.content))
        .filter(|s| s.len() > MIN_SUMMARY_LEN)
    {
        return Some(summary);
    }

    page.meta
        .iter()
        .find(|tag| tag.name == "description")
        .map(|tag| decode(&tag.content))
        .filter(|s| s.len() > MIN_SUMMARY_LEN)
}
