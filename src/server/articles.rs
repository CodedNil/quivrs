use crate::{
    server::{
        HTTP_CLIENT,
        embeddings::{cosine_similarity, generate_embeddings},
        llm_functions::run,
    },
    shared::{ArticleEntry, ArticleSource, StoredArticle},
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use futures::future::join_all;
use itertools::Itertools;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::LazyLock,
};
use tokio::fs;
use tracing::{error, info, warn};
use uuid::Uuid;

const SIMILARITY_THRESHOLD: f32 = 0.6;

pub const ARTICLES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("articles");

pub static DB: LazyLock<Database> = LazyLock::new(|| {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.redb".to_string());
    Database::create(url).unwrap()
});

pub async fn refresh_all_feeds() -> Result<()> {
    // Load all existing articles for similarity matching
    let mut articles = if let Ok(table) = DB.begin_read()?.open_table(ARTICLES_TABLE) {
        table
            .iter()?
            .flatten()
            .filter_map(|(k, v)| {
                Some((
                    Uuid::parse_str(k.value()).ok()?,
                    postcard::from_bytes::<StoredArticle>(v.value()).ok()?,
                ))
            })
            .collect()
    } else {
        HashMap::new()
    };
    // All existing URLs to avoid duplicates
    let existing_urls: HashSet<&String> = articles
        .values()
        .flat_map(|article| &article.sources)
        .map(|source| &source.url)
        .collect();

    // Scan all feeds in parallel, collect new entries
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.ron".to_string());
    let config_file: HashMap<String, String> =
        ron::from_str(&fs::read_to_string(&config_path).await?)
            .map_err(|e| anyhow!("Failed to read {config_path}: {e}"))?;

    let feeds: Vec<(String, String)> = config_file.into_iter().collect();

    info!("Scanning {} feeds", feeds.len());
    let new_entries: HashSet<ArticleSource> =
        join_all(feeds.into_iter().map(|(id, url)| async move {
            scan_feed(&url).await.unwrap_or_else(|err| {
                warn!(feed_id = %id, "Feed scan failed: {err:#}");
                vec![]
            })
        }))
        .await
        .into_iter()
        .flatten()
        .filter(|entry| !existing_urls.contains(&entry.url))
        .collect();

    if new_entries.is_empty() {
        info!("No new entries found");
        return Ok(());
    }

    // Generate embeddings for all new articles
    info!(
        "Generating embeddings for {} new articles...",
        new_entries.len()
    );
    let embeddings = generate_embeddings(
        &new_entries
            .iter()
            .map(|source| format!("{} {}", source.title, source.description))
            .collect::<Vec<_>>(),
    )
    .await
    .inspect_err(|e| error!("Batch embedding generation failed: {e}"))?;

    // Assign each new entry to an existing article (if similar) or create a new one
    for (source, embedding) in new_entries.into_iter().zip(embeddings) {
        if embedding.is_empty() {
            warn!(
                "Skipping article due to empty embedding vector for: {}",
                source.title
            );
            continue;
        }

        // Find the highest similarity match
        let highest_match = articles
            .values()
            .filter(|a| !a.embedding.is_empty())
            .filter(|a| {
                (source.published - a.sources[0].published).abs() <= chrono::TimeDelta::days(2)
            })
            .map(|a| {
                (
                    a.id,
                    &a.sources[0].title,
                    cosine_similarity(&embedding, &a.embedding),
                )
            })
            .max_by(|a, b| a.2.total_cmp(&b.2));

        let write_txn = DB.begin_write()?;
        {
            let mut table = write_txn.open_table(ARTICLES_TABLE)?;

            if let Some(&(article_id, existing_title, sim)) = highest_match
                .as_ref()
                .filter(|m| m.2 >= SIMILARITY_THRESHOLD)
            {
                info!(
                    "[MERGE] '{}' → '{}' (sim {sim:.2})",
                    source.title, existing_title
                );

                if let Some(article) = articles.get_mut(&article_id) {
                    article.sources.push(source);
                    article.updated_at = Utc::now();
                    article.entry = None; // Reset entry so regeneration targets it

                    table.insert(
                        &article_id.to_string().as_str(),
                        postcard::to_allocvec(article)?.as_slice(),
                    )?;
                }
            } else {
                let id = Uuid::new_v4();

                // Print the runner-up similarity
                if let Some((_, closest_title, sim)) = &highest_match {
                    info!(
                        "[NEW] '{}' - Highest {sim:.2} '{}'",
                        source.title, closest_title
                    );
                } else {
                    info!("[NEW] '{}'", source.title);
                }

                let new_article = StoredArticle {
                    id,
                    sources: vec![source],
                    estimated_liked: 0.0,
                    entry: None,
                    embedding,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                table.insert(
                    &id.to_string().as_str(),
                    postcard::to_allocvec(&new_article)?.as_slice(),
                )?;

                // Track internally so upcoming iterations can match against it
                articles.insert(id, new_article);
            }
        }
        write_txn.commit()?;
    }

    Ok(())
}

pub async fn regenerate_articles() -> Result<()> {
    if env::var("OPENROUTER").is_err() {
        warn!("OPENROUTER environment variable not set. Skipping generation.");
        return Ok(());
    }

    let targets: Vec<(Uuid, StoredArticle)> =
        if let Ok(table) = DB.begin_read()?.open_table(ARTICLES_TABLE) {
            table
                .iter()?
                .flatten()
                .filter_map(|(k, v)| {
                    let article = postcard::from_bytes::<StoredArticle>(v.value()).ok()?;
                    let id = Uuid::parse_str(k.value()).ok()?;
                    (article.entry.is_none() && article.sources.len() > 1).then_some((id, article))
                })
                .collect()
        } else {
            return Ok(());
        };

    if targets.is_empty() {
        return Ok(());
    }

    info!("Generating content for {} articles...", targets.len());

    for (id, mut article) in targets {
        // Long running await out of any DB transaction locks
        match generate_article_content(&article.sources).await {
            Ok(entry) => {
                info!("[GEN SUCCESS] '{}'", entry.title);
                article.entry = Some(entry);
                article.updated_at = Utc::now();

                let write_txn = DB.begin_write()?;
                {
                    let mut table = write_txn.open_table(ARTICLES_TABLE)?;
                    table.insert(
                        &id.to_string().as_str(),
                        postcard::to_allocvec(&article)?.as_slice(),
                    )?;
                }
                write_txn.commit()?;
            }
            Err(err) => {
                warn!(article_id = %id, "Generation failed: {err:#}");
            }
        }
    }

    Ok(())
}

async fn scan_feed(url_rss: &str) -> Result<Vec<ArticleSource>> {
    let content = HTTP_CLIENT.get(url_rss).send().await?.bytes().await?;
    let feed = feedparser_rs::parse(content.as_ref())?;

    // Write as url_rss.json
    let safe_filename = url_rss
        .replace("https://", "")
        .replace("http://", "")
        .replace(['/', ':'], "_");
    let file_path = format!("{safe_filename}.json");
    std::fs::create_dir_all("tmp").ok();
    std::fs::write(format!("tmp/{file_path}.json"), format!("{feed:#?}")).unwrap();

    let articles = feed
        .entries
        .into_iter()
        .filter_map(|entry| {
            let mut raw_url = entry
                .clone()
                .link
                .or_else(|| entry.links.first().map(|l| l.href.to_string()))?;

            // Get external url for reddit posts
            if raw_url.contains("reddit.com/r/") {
                // Look inside the summary/content string for the external article link
                // Reddit format: <span><a href="EXTERNAL_URL">[link]</a></span>
                let search_content = entry.summary.as_deref().unwrap_or("");

                if let Some(link_idx) = search_content.find("\">[link]</a>") {
                    // Walk backwards to find the start of the href attribute
                    if let Some(href_start) = search_content[..link_idx].rfind("href=\"") {
                        let start = href_start + 6; // Move past 'href="'
                        let external_url = &search_content[start..link_idx];

                        // Swap out the Reddit thread URL for the true external article URL
                        raw_url = external_url.to_string();
                    } else {
                        return None; // It's a discussion/text thread; skip it.
                    }
                } else {
                    return None; // No external link tag found; skip it.
                }
            }

            let url =
                match url_normalize::normalize_url(&raw_url, &url_normalize::Options::default()) {
                    Ok(u) => u,
                    Err(e) => {
                        error!("Failed to normalize URL '{}': {:?}", raw_url, e);
                        return None;
                    }
                };

            if url.contains("bbc.co.uk/iplayer") || url.contains("bbc.co.uk/sounds") {
                return None;
            }

            // Grab the primary image
            let image = entry
                .media_thumbnail
                .first()
                .map(|t| (t.url.to_string(), entry.media_title.clone()));

            // Reconstruct content blocks by joining them if multiple exist
            let description = if entry.content.is_empty() {
                entry.summary.unwrap_or_else(|| "NOT PROVIDED".into())
            } else {
                entry.content.into_iter().map(|c| c.value).join("\n")
            };

            Some(ArticleSource {
                url,
                title: entry.title.unwrap_or_else(|| "Untitled".into()),
                description,
                image,
                published: entry.published.unwrap_or_else(Utc::now),
            })
        })
        .collect();

    Ok(articles)
}

async fn generate_article_content(sources: &[ArticleSource]) -> Result<ArticleEntry> {
    let fetches = sources.iter().map(|source| async move {
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
                .iter()
                .map(|(url, alt)| (url.as_str(), alt.as_deref()))
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
        "Synthesise all sources into a single cohesive article in the output json_schema. Use a few web searchs for the title of each source to gather the latest information. EVERYTHING must be entirely factual and based on the sources provided, no assumptions or guesswork.\n\n{articles_content}"
    );

    run::<ArticleEntry>(&context)
        .await
        .map_err(|e| anyhow!("Article generation failed: {e}"))
}
