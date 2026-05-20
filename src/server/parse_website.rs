use crate::{server::HTTP_CLIENT, shared::ArticleSource};
use anyhow::{Result, anyhow};
use chrono::Utc;
use itertools::Itertools;
use postcard::{from_bytes, to_allocvec};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt::Write};
use tokio::fs;
use url::Url;

/// Downloads the full webpage and parses title, summary, content, and images on the source.
pub async fn fetch_source_content(url: String) -> Result<ArticleSource> {
    let cache_path = {
        let hash = Sha256::digest(url.as_bytes())
            .iter()
            .fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            });
        std::env::temp_dir().join("quivrs").join(hash)
    };

    if let Ok(bytes) = fs::read(&cache_path).await
        && let Ok(source) = from_bytes::<ArticleSource>(&bytes)
    {
        return Ok(source);
    }

    let source = extract_source_content(url).await?;

    if let Ok(bytes) = to_allocvec(&source) {
        let dir = cache_path.parent().unwrap();
        if fs::create_dir_all(dir).await.is_ok() {
            let _ = fs::write(&cache_path, bytes).await;
        }
    }

    Ok(source)
}

async fn extract_source_content(url: String) -> Result<ArticleSource> {
    let base_url = Url::parse(&url).ok();
    let html = HTTP_CLIENT.get(&url).send().await?.text().await?;
    let options = rs_trafilatura::Options {
        include_comments: false,
        include_tables: true,
        include_images: true,
        include_links: false,
        include_formatting: false,
        favor_recall: true,
        deduplicate: true,
        max_extracted_len: 22000,
        ..Default::default()
    };
    let extracted =
        rs_trafilatura::extract_with_options(&html, &options).map_err(|e| anyhow!("{e}"))?;

    let mut seen_images = HashSet::new();
    let resolve_url = |img_url: &str| {
        base_url.as_ref().map_or_else(
            || img_url.to_string(),
            |base| {
                base.join(img_url)
                    .map_or_else(|_| img_url.to_string(), |u| u.to_string())
            },
        )
    };

    let images = extracted
        .metadata
        .image
        .map(|img_url| (resolve_url(&img_url), String::new()))
        .into_iter()
        .chain(
            extracted
                .images
                .iter()
                .sorted_by_key(|img| !img.is_hero)
                .map(|img| {
                    (
                        resolve_url(&img.src),
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
        title: html_escape::decode_html_entities(&extracted.metadata.title.unwrap_or_default())
            .into_owned(),
        summary: html_escape::decode_html_entities(
            &extracted.metadata.description.unwrap_or_default(),
        )
        .into_owned(),
        content: extracted.content_text,
        tags: extracted.metadata.tags,
        images,
        published: extracted.metadata.date.unwrap_or_else(Utc::now),
    })
}
