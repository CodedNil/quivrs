use super::{PageData, jsonld_values};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use scraper::{Html, Selector};
use std::sync::LazyLock;

static SEL_TIME_DATETIME: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("time[datetime]").unwrap());
static SEL_VISIBLE_DATE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(
        ".date, .post-date, .entry-date, .published-at, #date_posted, \
         article strong, .post-meta, .entry-meta",
    )
    .unwrap()
});

pub fn parse(page: &PageData) -> Option<DateTime<Utc>> {
    // 1. JSON-LD
    if let Some(date) = page.jsonld.iter().flat_map(jsonld_values).find_map(|obj| {
        let key = obj
            .get("datePublished")
            .or_else(|| obj.get("dateModified"))
            .or_else(|| obj.get("published_at"))
            .or_else(|| obj.get("updated_at"))?;
        parse_any(key.as_str()?)
    }) {
        return Some(date);
    }

    // 2. Meta tags
    if let Some(date) = page.meta.iter().find_map(|tag| {
        matches!(
            tag.name.as_str(),
            "og:article:published_time"
                | "article:published_time"
                | "parsely-pub-date"
                | "last-modified"
        )
        .then(|| parse_any(&tag.content))?
    }) {
        return Some(date);
    }

    let doc = Html::parse_document(&page.html);

    // 3. <time datetime="..."> elements
    if let Some(date) = doc
        .select(&SEL_TIME_DATETIME)
        .filter_map(|el| el.value().attr("datetime"))
        .find_map(parse_any)
    {
        return Some(date);
    }

    // 4. Visible date elements and common inline containers
    doc.select(&SEL_VISIBLE_DATE)
        .map(|el| el.text().collect::<String>())
        .find_map(|text| parse_text(text.trim()))
}

/// Parse a date string that may have trailing metadata (e.g. "June 6, 2026 • 8 Notes").
fn parse_text(s: &str) -> Option<DateTime<Utc>> {
    parse_any(s).or_else(|| {
        // Split on common delimiters and try the first part
        [" • ", " | ", " · ", " — ", " – ", " - ", "\t", "\n"]
            .iter()
            .find_map(|sep| {
                s.split(sep)
                    .next()
                    .and_then(|first| parse_any(first.trim()))
            })
    })
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
                .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
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
