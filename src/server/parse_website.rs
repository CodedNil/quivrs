use crate::{server::HTTP_CLIENT, shared::ArticleSource};
use anyhow::{Result, anyhow};
use chrono::Utc;
use itertools::Itertools;
use std::collections::HashSet;

/// Downloads the full webpage and parses title, summary, content, and images on the source.
pub async fn fetch_source_content(url: String) -> Result<ArticleSource> {
    let html = HTTP_CLIENT.get(&url).send().await?.text().await?;
    let options = rs_trafilatura::Options {
        include_comments: false,
        include_tables: true,
        include_images: true,
        include_links: false,
        include_formatting: false,
        favor_recall: true,
        deduplicate: true,
        max_extracted_len: 20000,
        ..Default::default()
    };
    let extracted =
        rs_trafilatura::extract_with_options(&html, &options).map_err(|e| anyhow!("{e}"))?;

    let mut seen_images = HashSet::new();
    let images = extracted
        .metadata
        .image
        .map(|url| {
            (
                url.split('?').next().unwrap_or("").to_string(),
                String::new(),
            )
        })
        .into_iter()
        .chain(
            extracted
                .images
                .iter()
                .sorted_by_key(|img| !img.is_hero)
                .map(|img| {
                    (
                        img.src.split('?').next().unwrap_or("").to_string(),
                        img.caption
                            .as_deref()
                            .or(img.alt.as_deref())
                            .unwrap_or("")
                            .to_string(),
                    )
                }),
        )
        .filter(|(url, _)| {
            !url.is_empty() && !url.contains("placeholder") && seen_images.insert(url.clone())
        })
        .map(|(url, caption)| format!("{url}|{caption}"))
        .collect();

    Ok(ArticleSource {
        url: url.clone(),
        title: extracted.metadata.title.unwrap_or_default(),
        summary: extracted.metadata.description.unwrap_or_default(),
        content: extracted.content_text,
        images,
        published: extracted.metadata.date.unwrap_or_else(Utc::now),
    })
}
