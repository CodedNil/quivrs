use super::{PageData, jsonld_values};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;

static SEL_TIME_DATETIME: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("time[datetime]").unwrap());
static SEL_VISIBLE_DATE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".date, .post-date, .entry-date, .published-at, #date_posted").unwrap()
});

pub fn parse(page: &PageData) -> Option<DateTime<Utc>> {
    if let Some(date) = page.jsonld.iter().flat_map(jsonld_values).find_map(|obj| {
        obj.get("datePublished")
            .or_else(|| obj.get("dateModified"))
            .or_else(|| obj.get("published_at"))
            .or_else(|| obj.get("updated_at"))
            .and_then(Value::as_str)
            .and_then(parse_any)
    }) {
        return Some(date);
    }

    if let Some(date) = page
        .meta
        .iter()
        .find(|tag| {
            matches!(
                tag.name.as_str(),
                "og:article:published_time" | "article:published_time" | "last-modified"
            )
        })
        .and_then(|tag| parse_any(&tag.content))
    {
        return Some(date);
    }

    let doc = Html::parse_document(&page.html);
    if let Some(date) = doc
        .select(&SEL_TIME_DATETIME)
        .filter_map(|el| el.value().attr("datetime"))
        .find_map(parse_any)
    {
        return Some(date);
    }

    doc.select(&SEL_VISIBLE_DATE)
        .map(|el| el.text().collect::<String>())
        .find_map(|text| parse_any(text.trim()))
}

fn parse_any(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_rfc3339(&format!("{s}:00")))
        .or_else(|_| DateTime::parse_from_rfc2822(s))
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z"))
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|date| date.and_utc())
        })
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%d %b %Y")
                .or_else(|_| NaiveDate::parse_from_str(s, "%d %B %Y"))
                .or_else(|_| NaiveDate::parse_from_str(s, "%B %d, %Y"))
                .or_else(|_| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|date| date.and_utc())
        })
}
