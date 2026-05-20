use crate::{
    server::{
        database,
        embeddings::{classify, cosine_similarity, generate_article_embeddings},
        llm_functions::run,
        parse_feed::scan_feed,
        parse_website::fetch_source_content,
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

const SIMILARITY_THRESHOLD: f32 = 0.7;

pub async fn refresh_all_feeds() -> Result<()> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.ron".to_string());
    let config_file: HashMap<String, String> =
        ron::from_str(&fs::read_to_string(&config_path).await?)
            .map_err(|e| anyhow!("Failed to read {config_path}: {e}"))?;

    info!("Scanning {} feeds", config_file.len());
    let candidate_urls: HashSet<String> =
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

    let new_urls = database::filter_new_urls(&candidate_urls).await?;
    if new_urls.is_empty() {
        info!("No new entries found");
        return Ok(());
    }

    info!("Fetching content for {} new articles...", new_urls.len());
    let fetched: Vec<ArticleSource> = join_all(new_urls.into_iter().map(fetch_source_content))
        .await
        .into_iter()
        .filter_map(|r| r.map_err(|e| warn!("Failed to fetch article: {e:#}")).ok())
        .collect();

    let (new_entries, dismissed): (Vec<_>, Vec<_>) = fetched.into_iter().partition(|s| {
        !s.title.is_empty() && !s.summary.is_empty() && s.title != s.summary && s.summary.len() > 20
    });
    let dismissed_urls: Vec<String> = dismissed.into_iter().map(|s| s.url).collect();
    database::mark_urls_dismissed(&dismissed_urls).await?;

    if new_entries.is_empty() {
        return Ok(());
    }

    info!(
        "Generating embeddings for {} new articles...",
        new_entries.len()
    );
    let (similarity_embeddings, classification_embeddings) =
        generate_article_embeddings(&new_entries)
            .await
            .inspect_err(|e| error!("Batch embedding generation failed: {e}"))?;

    for (source, (sim_emb, cls_emb)) in new_entries.into_iter().zip(
        similarity_embeddings
            .into_iter()
            .zip(classification_embeddings),
    ) {
        if sim_emb.is_empty() || cls_emb.is_empty() {
            warn!("Skipping article due to empty embedding: {}", source.title);
            continue;
        }

        // Find the highest similarity match
        let candidates = database::get_embedding_candidates(
            (source.published - TimeDelta::days(2)).timestamp(),
            (source.published + TimeDelta::days(2)).timestamp(),
        )
        .await?;

        let mut best: Option<(uuid::Uuid, &str, f32)> = None;
        let mut highest: Option<(&str, f32)> = None;
        for c in candidates
            .iter()
            .filter(|c| !c.embedding_similarity.is_empty())
        {
            let score = cosine_similarity(&sim_emb, &c.embedding_similarity);
            if highest.is_none_or(|(_, s)| score > s) {
                highest = Some((&c.title, score));
            }
            if score >= SIMILARITY_THRESHOLD && best.is_none_or(|(_, _, s)| score > s) {
                best = Some((c.id, &c.title, score));
            }
        }

        if let Some((article_id, existing_title, score)) = best {
            info!(
                "[MERGE] '{}' → '{}' (score {score:.2})",
                source.title, existing_title
            );
            database::merge_into_article(article_id, &source).await?;
        } else {
            if let Some((closest_title, sim)) = highest {
                info!(
                    "[NEW] '{}' - Highest {sim:.2} '{}'",
                    source.title, closest_title
                );
            } else {
                info!("[NEW] '{}'", source.title);
            }
            if let Ok((article_type, category)) = classify(&cls_emb).await {
                database::insert_article(&source, &sim_emb, &cls_emb, article_type, category)
                    .await?;
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
    let articles_content = sources
        .into_iter()
        .enumerate()
        .map(|(i, source)| {
            let images: String = source
                .images
                .iter()
                .map(|img_str| {
                    let (src, caption) = img_str.split_once('|').unwrap_or((img_str, ""));
                    if caption.is_empty() {
                        format!("[{src}]")
                    } else {
                        format!("[{src}, {caption}]")
                    }
                })
                .join(" ");

            format!(
                "Source {} - Title: {} - URL: {} - Images: {} - Content: {}",
                i + 1,
                source.title,
                source.url,
                images,
                source.content.replace('\n', " ")
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
