use crate::{
    server::{
        HTTP_CLIENT, database,
        embeddings::{classify, cosine_similarity, generate_embeddings},
        llm_functions::run,
        parse_feed::scan_feed,
    },
    shared::{ArticleEntry, ArticleSource},
};

use anyhow::{Result, anyhow};
use chrono::TimeDelta;
use futures::{StreamExt, future::join_all, stream};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    env,
};
use tokio::fs;
use tracing::{error, info, warn};

const SIMILARITY_THRESHOLD: f32 = 0.6;

pub async fn refresh_all_feeds() -> Result<()> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.ron".to_string());
    let config_file: HashMap<String, String> =
        ron::from_str(&fs::read_to_string(&config_path).await?)
            .map_err(|e| anyhow!("Failed to read {config_path}: {e}"))?;

    info!("Scanning {} feeds", config_file.len());
    let all_entries: HashSet<ArticleSource> =
        join_all(config_file.into_iter().map(|(id, url)| async move {
            scan_feed(&url).await.unwrap_or_else(|err| {
                warn!(feed_id = %id, "Feed scan failed: {err:#}");
                vec![]
            })
        }))
        .await
        .into_iter()
        .flatten()
        .collect();

    let mut new_entries = Vec::new();
    for entry in all_entries {
        if !database::url_exists(&entry.url).await {
            new_entries.push(entry);
        }
    }

    if new_entries.is_empty() {
        info!("No new entries found");
        return Ok(());
    }

    info!(
        "Generating embeddings for {} new articles...",
        new_entries.len()
    );
    let embeddings = generate_embeddings(
        &new_entries
            .iter()
            .map(|s| format!("{} {} {}", s.url, s.title, s.summary))
            .collect::<Vec<_>>(),
    )
    .await
    .inspect_err(|e| error!("Batch embedding generation failed: {e}"))?;

    for (source, embedding) in new_entries.into_iter().zip(embeddings) {
        if embedding.is_empty() {
            warn!("Skipping article due to empty embedding: {}", source.title);
            continue;
        }

        // Find the highest similarity match
        let candidates = database::get_embedding_candidates(
            (source.published - TimeDelta::days(2)).timestamp(),
            (source.published + TimeDelta::days(2)).timestamp(),
        )
        .await?;
        let highest_match = candidates
            .iter()
            .filter(|c| !c.embedding.is_empty())
            .map(|c| (c.id, &c.title, cosine_similarity(&embedding, &c.embedding)))
            .max_by(|a, b| a.2.total_cmp(&b.2));

        if let Some((article_id, existing_title, sim)) =
            highest_match.filter(|m| m.2 >= SIMILARITY_THRESHOLD)
        {
            info!(
                "[MERGE] '{}' → '{}' (sim {sim:.2})",
                source.title, existing_title
            );
            database::merge_into_article(article_id, &source, &embedding).await?;
        } else {
            if let Some((_, closest_title, sim)) = &highest_match {
                info!(
                    "[NEW] '{}' - Highest {sim:.2} '{}'",
                    source.title, closest_title
                );
            } else {
                info!("[NEW] '{}'", source.title);
            }
            if let Ok((article_type, category)) = classify(&embedding).await {
                database::insert_article(&source, &embedding, article_type, category).await?;
            }
        }
    }

    Ok(())
}

pub async fn regenerate_articles() -> Result<()> {
    if env::var("OPENROUTER").is_err() {
        warn!("OPENROUTER environment variable not set. Skipping generation.");
        return Ok(());
    }

    let targets = database::get_regeneration_targets().await?;
    if targets.is_empty() {
        return Ok(());
    }

    info!("Generating content for {} articles...", targets.len());

    let mut article_stream = stream::iter(targets)
        .map(|(id, sources)| async move { (id, generate_article_content(sources).await) })
        .buffer_unordered(5);

    while let Some((id, result)) = article_stream.next().await {
        match result {
            Ok(entry) => {
                if let Err(err) = database::save_article_entry(id, &entry).await {
                    warn!(article_id = %id, "Failed to save article to database: {err:#}");
                }
            }
            Err(err) => {
                warn!(article_id = %id, "Generation failed: {err:#}");
            }
        }
    }

    Ok(())
}

async fn generate_article_content(sources: Vec<ArticleSource>) -> Result<ArticleEntry> {
    let fetches = sources.into_iter().map(|source| async move {
        let content = async {
            let html = HTTP_CLIENT.get(&source.url).send().await?.text().await?;
            let options = rs_trafilatura::Options {
                include_comments: false,
                include_tables: true,
                include_images: true,
                include_links: true,
                favor_recall: true,
                include_formatting: true,
                target_language: Some("en".to_string()),
                deduplicate: true,
                max_extracted_len: 30000,
                page_type: Some(rs_trafilatura::page_type::PageType::Article),
                ..Default::default()
            };
            rs_trafilatura::extract_with_options(&html, &options).map_err(|e| anyhow!("{e}"))
        }
        .await;

        match content {
            Ok(extracted) => Ok((source, extracted)),
            Err(err) => {
                warn!(url = %source.url, "Failed to fetch content: {err:#}");
                Err(err)
            }
        }
    });

    let successful_fetches = join_all(fetches)
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let articles_content = successful_fetches
        .into_iter()
        .enumerate()
        .map(|(i, (source, content))| {
            let mut images_found = HashSet::new();
            let images: String = source
                .image
                .as_deref()
                .map(|url| (url, source.image_description.as_deref()))
                .into_iter()
                .chain(
                    content
                        .images
                        .iter()
                        .sorted_by_key(|img| !img.is_hero)
                        .map(|i| (i.src.as_str(), i.caption.as_deref().or(i.alt.as_deref()))),
                )
                .filter(|(src, _)| {
                    images_found.insert(src.to_string()) && !src.contains("placeholder")
                })
                .map(|(src, alt)| {
                    let clean_src = src.split('?').next().unwrap_or(src);
                    alt.filter(|s| !s.is_empty()).map_or_else(
                        || format!("[{clean_src}]"),
                        |text| format!("[{clean_src}, {text}]"),
                    )
                })
                .join(" ");

            format!(
                "Source {} - Title: {} - URL: {} - Content: {} - Images: {}",
                i + 1,
                source.title,
                source.url,
                content.content_text.replace('\n', " "),
                images
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let context = format!(
        "Synthesise all sources into a single cohesive article in the output json_schema. \
         Use a few web searches for the title of each source to gather the latest information. \
         EVERYTHING must be entirely factual and based on the sources provided, no assumptions or guesswork.\n\n{articles_content}"
    );

    run::<ArticleEntry>(&context)
        .await
        .map_err(|e| anyhow!("Article generation failed: {e}"))
}
