use super::{
    HTTP_CLIENT,
    embeddings::{cosine_similarity, get_embedding},
    llm_functions::run,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use feed_rs::parser;
use futures::future::join_all;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::{Arc, LazyLock},
};
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

const SIMILARITY_THRESHOLD: f32 = 0.88;

pub const ARTICLES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("articles");
const SOURCE_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("source_index");

pub static DB: LazyLock<Database> = LazyLock::new(|| {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.redb".to_string());
    Database::create(url).unwrap()
});

type FeedConfigFile = HashMap<String, String>;

#[derive(Serialize, Deserialize)]
struct StoredArticle {
    id: Uuid,
    sources: Vec<ArticleSource>,
    entry: ArticleEntry,
    embedding: Vec<f32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ArticleSource {
    url: String,
    title: String,
    published: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, JsonSchema, Default)]
struct ArticleEntry {
    /// Article's title, kept concise and descriptive
    title: String,
    /// Short information summary with no newlines, a few sentences max
    description: String,
    /// Full article content as rendered HTML; use figure/figcaption for images, include inline links, well-structured paragraphs; place first image after first paragraph as thumbnail
    full_content: String,
    /// Estimated user interest 0.0-1.0; higher for technically deep, novel, or well-written content; lower for clickbait, marketing, or low-effort content
    estimated_liked: f32,
    /// Descriptive tags (e.g. technology, ai, machine-learning), lowercase, hyphen-separated
    tags: Vec<String>,
}

pub async fn refresh_all_feeds() -> Result<()> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "feeds.json".to_string());
    let config_str = fs::read_to_string(&config_path).await?;
    let config_file: FeedConfigFile = serde_json::from_str(&config_str)
        .map_err(|e| anyhow!("Failed to read {config_path}: {e}"))?;

    let feeds: Vec<(String, String)> = config_file
        .iter()
        .map(|(id, url)| (id.clone(), url.clone()))
        .collect();

    info!("Scanning {} feeds", feeds.len());

    // SOURCE_INDEX_TABLE is the persistent set of all processed source URLs
    let known_sources: Arc<HashSet<String>> = Arc::new({
        let read_txn = DB.begin_read()?;
        let mut set = HashSet::new();
        if let Ok(table) = read_txn.open_table(SOURCE_INDEX_TABLE) {
            for item in table.iter()? {
                let (k, _) = item?;
                set.insert(k.value().to_string());
            }
        }
        set
    });

    // Scan all feeds in parallel, collect new entries
    let new_entries: Vec<ArticleSource> = join_all(feeds.into_iter().map(|(feed_id, url_rss)| {
        let known = Arc::clone(&known_sources);
        async move {
            match scan_feed(&feed_id, &url_rss, &known).await {
                Ok(entries) => entries,
                Err(err) => {
                    warn!(feed_id = %feed_id, "Feed scan failed: {err:#}");
                    vec![]
                }
            }
        }
    }))
    .await
    .into_iter()
    .flatten()
    .collect();

    if new_entries.is_empty() {
        info!("No new entries found");
        return Ok(());
    }

    info!("Embedding {} new entries", new_entries.len());

    // Embeddings always run (local model)
    let embeddings: Vec<Vec<f32>> =
        join_all(new_entries.iter().map(|s| get_embedding(s.title.clone())))
            .await
            .into_iter()
            .map(|r| {
                r.unwrap_or_else(|e| {
                    warn!("Embedding failed: {e}");
                    vec![]
                })
            })
            .collect();

    // Load all existing articles for similarity matching
    let mut articles: HashMap<Uuid, StoredArticle> = {
        let read_txn = DB.begin_read()?;
        let mut map = HashMap::new();
        if let Ok(table) = read_txn.open_table(ARTICLES_TABLE) {
            for item in table.iter()? {
                let (k, v) = item?;
                if let Ok(article) = postcard::from_bytes::<StoredArticle>(v.value())
                    && let Ok(id) = Uuid::parse_str(k.value())
                {
                    map.insert(id, article);
                }
            }
        }
        map
    };

    // Assign each new entry to an existing article (if similar) or create a new one
    let mut articles_to_generate: Vec<Uuid> = Vec::new();
    let mut new_source_index: HashMap<String, Uuid> = HashMap::new();

    for (source, embedding) in new_entries.into_iter().zip(embeddings.into_iter()) {
        let similar = articles
            .values()
            .filter(|a| !a.embedding.is_empty() && !embedding.is_empty())
            .filter_map(|a| {
                let sim = cosine_similarity(&embedding, &a.embedding);
                (sim >= SIMILARITY_THRESHOLD).then_some((a.id, &a.entry.title, sim))
            })
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, title, sim)| (id, title.clone(), sim));

        if let Some((article_id, existing_title, sim)) = similar {
            info!(
                "[MERGE] '{}' → '{}' (sim {sim:.2})",
                source.title, existing_title
            );
            let article = articles.get_mut(&article_id).unwrap();
            article.sources.push(source.clone());
            article.updated_at = Utc::now();
            if !articles_to_generate.contains(&article_id) {
                articles_to_generate.push(article_id);
            }
            new_source_index.insert(source.url, article_id);
        } else {
            let id = Uuid::new_v4();
            info!("[NEW] '{}'", source.title);
            articles.insert(
                id,
                StoredArticle {
                    id,
                    sources: vec![source.clone()],
                    entry: ArticleEntry::default(),
                    embedding,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
            articles_to_generate.push(id);
            new_source_index.insert(source.url, id);
        }
    }

    // Generate LLM content for changed articles
    if env::var("OPENROUTER").is_ok() {
        info!(
            "Generating content for {} articles",
            articles_to_generate.len()
        );

        let generate_tasks: Vec<_> = articles_to_generate
            .iter()
            .filter_map(|id| articles.get(id).map(|a| (*id, a.sources.clone())))
            .map(|(id, sources): (Uuid, Vec<ArticleSource>)| async move {
                match generate_article_content(&sources).await {
                    Ok(entry) => {
                        info!("[GEN   ] '{}'", entry.title);
                        Some((id, entry))
                    }
                    Err(err) => {
                        warn!(article_id = %id, "Generation failed: {err:#}");
                        None
                    }
                }
            })
            .collect();

        for (id, entry) in join_all(generate_tasks).await.into_iter().flatten() {
            if let Some(article) = articles.get_mut(&id) {
                article.entry = entry;
            }
        }
    }

    // Persist new/updated articles and source index entries
    let write_txn = DB.begin_write()?;
    {
        let mut articles_table = write_txn.open_table(ARTICLES_TABLE)?;
        for id in &articles_to_generate {
            if let Some(article) = articles.get(id) {
                articles_table.insert(
                    id.to_string().as_str(),
                    postcard::to_allocvec(article)?.as_slice(),
                )?;
            }
        }
    }
    {
        let mut source_table = write_txn.open_table(SOURCE_INDEX_TABLE)?;
        for (url, article_id) in &new_source_index {
            source_table.insert(url.as_str(), article_id.to_string().as_bytes())?;
        }
    }
    write_txn.commit()?;

    info!(
        "Done: {} articles updated, {} sources indexed",
        articles_to_generate.len(),
        new_source_index.len()
    );
    Ok(())
}

async fn scan_feed(
    feed_id: &str,
    url_rss: &str,
    known_sources: &HashSet<String>,
) -> Result<Vec<ArticleSource>> {
    let content = HTTP_CLIENT.get(url_rss).send().await?.bytes().await?;
    let fetched = parser::parse(content.as_ref())?;

    let new_entries: Vec<ArticleSource> = fetched
        .entries
        .iter()
        .filter_map(|entry| {
            let url = entry.links.first()?.href.clone();
            if known_sources.contains(&url) {
                return None;
            }
            let title = entry
                .title
                .as_ref()
                .map_or_else(|| "Untitled".to_string(), |t| t.content.clone());
            Some(ArticleSource {
                url,
                title,
                published: entry.published.unwrap_or_else(Utc::now),
            })
        })
        .collect();

    info!(
        "Feed {feed_id}: {}/{} entries new",
        new_entries.len(),
        fetched.entries.len()
    );
    Ok(new_entries)
}

async fn generate_article_content(sources: &[ArticleSource]) -> Result<ArticleEntry> {
    let fetches = sources.iter().map(|source| async move {
        let content = async {
            let html = HTTP_CLIENT.get(&source.url).send().await?.text().await?;
            html2text::from_read(html.as_bytes(), 80).map_err(|e| anyhow!("{e}"))
        }
        .await
        .unwrap_or_else(|err| {
            warn!(url = %source.url, "Failed to fetch content: {err:#}");
            String::new()
        });

        (source, content)
    });

    let context = join_all(fetches)
        .await
        .into_iter()
        .enumerate()
        .map(|(i, (source, content))| {
            let mut block = format!(
                "--- Source {} ---\nTitle: {}\nURL: {}",
                i + 1,
                source.title,
                source.url
            );

            if !content.is_empty() {
                block.push_str("\nContent:\n");
                block.push_str(&content);
            }

            block
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    run::<ArticleEntry>(&context, "Generate a consolidated article from the provided sources. Synthesise all sources into a single cohesive piece.")
        .await
        .map_err(|e| anyhow!("Article generation failed: {e}"))
}
