use crate::server::parsers::usable_article_url;
use anyhow::Result;
use chrono::{DateTime, Utc};
use duckduckgo::{browser::Browser, user_agents::get};
use std::collections::HashSet;
use tracing::warn;
use url::Url;

const MAX_SEARCH_RESULTS: usize = 8;
const DDG_REGION: &str = "us-en";
const DDG_SAFE_SEARCH: bool = true;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub url: String,
    pub published: Option<DateTime<Utc>>,
}

/// Searches `DuckDuckGo` news results and returns article URLs with result dates.
pub async fn search_article_urls(query: &str) -> Result<Vec<SearchResult>> {
    let browser = Browser::new();
    let results = browser
        .news(
            query,
            DDG_REGION,
            DDG_SAFE_SEARCH,
            Some(MAX_SEARCH_RESULTS),
            get("firefox").unwrap_or("Mozilla/5.0"),
        )
        .await?;

    let mut seen = HashSet::new();
    let mut urls = Vec::new();

    for result in results {
        let Some(url) = usable_article_url(&result.url).filter(|url| {
            Url::parse(url).is_ok_and(|parsed| {
                !parsed.domain().is_some_and(|domain| {
                    domain.ends_with("duckduckgo.com")
                        || domain.ends_with("youtube.com")
                        || domain.ends_with("youtu.be")
                }) && !std::path::Path::new(parsed.path())
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            })
        }) else {
            continue;
        };

        if seen.insert(url.clone()) {
            urls.push(SearchResult {
                published: {
                    DateTime::parse_from_rfc3339(&result.date)
                        .map(|date| date.with_timezone(&Utc))
                        .ok()
                },
                url,
            });
        }
    }

    if urls.is_empty() {
        warn!("Web search returned no article URLs for query: {query}");
    }

    Ok(urls)
}
