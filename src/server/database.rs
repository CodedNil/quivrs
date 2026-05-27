use crate::shared::{Article, ArticleStatus, Category, PendingSource, Rating};
use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};
use surrealdb::{
    Surreal,
    engine::local::{Db, SurrealKv},
    types::{RecordId, SurrealValue},
};
use tracing::info;
use uuid::Uuid;

pub static DB: LazyLock<Surreal<Db>> = LazyLock::new(Surreal::init);

const PREFERENCE_NEIGHBOURS: u32 = 64;
const SIMILAR_SOURCE_NEIGHBOURS: u32 = 50;
const KNN_EF: u32 = 200;

#[derive(Clone, SurrealValue)]
pub struct LabelEmbeddingRecord {
    pub id: RecordId,
    pub label_group: String,
    pub label_value: String,
    pub hash: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

#[derive(SurrealValue)]
struct LabelHashRow {
    id: RecordId,
    hash: String,
}

#[derive(SurrealValue)]
pub struct LabelScore {
    pub label_group: String,
    pub label_value: String,
    pub similarity: f32,
}

#[derive(SurrealValue)]
struct GroupedCount {
    category: Category,
    count: i64,
}

#[derive(SurrealValue)]
pub struct StaleEmbeddingRecord {
    pub id: RecordId,
    pub embedding_text: String,
}

#[derive(Clone, SurrealValue)]
pub struct EmbeddingUpdate {
    pub id: RecordId,
    pub embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct SimilarSource {
    #[serde(flatten)]
    source: PendingSource,
    dist: f32,
}

#[derive(SurrealValue)]
struct PreferenceRow {
    rating: Rating,
    dist: f32,
}

#[derive(SurrealValue)]
struct ItemRatingRow {
    id: RecordId,
    rating: Rating,
}

fn record_key_to_string(id: RecordId) -> String {
    match id.key {
        surrealdb::types::RecordIdKey::String(s) => s,
        surrealdb::types::RecordIdKey::Number(n) => n.to_string(),
        surrealdb::types::RecordIdKey::Uuid(u) => u.to_string(),
        other => format!("{other:?}"),
    }
}

pub async fn init() -> Result<()> {
    DB.connect::<SurrealKv>("quivrs.db").await?;
    DB.use_ns("quivrs").use_db("quivrs").await?;

    // Simple migration runner
    run_migrations().await?;

    Ok(())
}

async fn run_migrations() -> Result<()> {
    // Ensure migrations table exists
    DB.query("DEFINE TABLE IF NOT EXISTS migrations SCHEMALESS;")
        .await?
        .check()?;

    let mut dir = tokio::fs::read_dir("migrations").await?;
    let mut entries = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext == "surql" || ext == "sql" {
            entries.push(entry.path());
        }
    }
    entries.sort();

    for path in entries {
        let name = path.file_name().unwrap().to_str().unwrap();
        let check: Option<surrealdb::types::Value> = DB.select(("migrations", name)).await?;

        if check.is_none() {
            info!("Running migration: {}", name);
            let sql = tokio::fs::read_to_string(&path).await?;
            DB.query(sql).await?.check()?;
            DB.create::<Option<surrealdb::types::Value>>(("migrations", name))
                .content(serde_json::json!({ "executed_at": Utc::now() }))
                .await?;
        }
    }

    Ok(())
}

/// Checks if a URL has already been processed, removes if they have.
pub async fn filter_new_urls(urls: &HashSet<String>) -> Result<Vec<String>> {
    if urls.is_empty() {
        return Ok(Vec::new());
    }

    let mut res = DB
        .query("SELECT VALUE id FROM article_urls WHERE id IN $urls")
        .bind((
            "urls",
            urls.iter()
                .map(|u| RecordId::new("article_urls", u.as_str()))
                .collect::<Vec<_>>(),
        ))
        .await?;

    let existing_ids: Vec<RecordId> = res.take(0)?;
    let existing: HashSet<String> = existing_ids.into_iter().map(record_key_to_string).collect();

    Ok(urls
        .iter()
        .filter(|u| !existing.contains(*u))
        .cloned()
        .collect())
}

/// Records URLs as permanently skipped so they are never re-fetched.
pub async fn mark_urls_dismissed(urls: &[String]) -> Result<()> {
    if urls.is_empty() {
        return Ok(());
    }

    DB.query(
        "FOR $url IN $urls {
             UPSERT type::record('article_urls', $url) CONTENT {} RETURN NONE;
         };",
    )
    .bind(("urls", urls.to_vec()))
    .await?
    .check()?;

    Ok(())
}

/// Stores newly found sources and records their URLs as processed.
pub async fn insert_sources(sources: Vec<PendingSource>) -> Result<()> {
    if sources.is_empty() {
        return Ok(());
    }

    DB.query(
        "BEGIN TRANSACTION;
         FOR $source IN $sources {
             UPSERT pending_sources CONTENT $source WHERE url = $source.url RETURN NONE;
             UPSERT type::record('article_urls', $source.url) CONTENT {} RETURN NONE;
         };
         COMMIT TRANSACTION;",
    )
    .bind(("sources", sources))
    .await?
    .check()?;

    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub async fn set_article_status(id: Uuid, status: ArticleStatus) -> Result<()> {
    let binned_at = (status == ArticleStatus::Binned).then(Utc::now);
    DB.query(
        "UPDATE type::record('user_articles', $id)
         SET status = $status, binned_at = $binned_at RETURN NONE",
    )
    .bind(("id", id))
    .bind(("status", status))
    .bind(("binned_at", binned_at))
    .await?
    .check()?;
    Ok(())
}

/// Sets a user rating for an article.
pub async fn set_rating(id: Uuid, rating: Rating) -> Result<()> {
    DB.query("UPDATE type::record('user_articles', $id) SET rating = $rating RETURN NONE")
        .bind(("id", id))
        .bind(("rating", rating))
        .await?
        .check()?;
    Ok(())
}

/// Retrieves all articles with their user-specific status and ratings.
pub async fn get_user_articles() -> Result<Vec<Article>> {
    let mut res = DB
        .query("SELECT *, record::id(id) AS id FROM user_articles ORDER BY published DESC")
        .await?;
    let articles: Vec<Article> = res.take(0)?;
    Ok(articles)
}

/// Sets a rating for a generic item (source, feed, or tag).
pub async fn set_item_rating(key: &str, rating: Rating) -> Result<()> {
    DB.query(
        "UPSERT type::record('item_ratings', $key)
         CONTENT { rating: $rating } RETURN NONE",
    )
    .bind(("key", key.to_string()))
    .bind(("rating", rating))
    .await?
    .check()?;
    Ok(())
}

pub async fn get_label_hashes() -> Result<HashMap<String, String>> {
    let mut rows = DB.query("SELECT id, hash FROM label_embeddings").await?;

    let rows: Vec<LabelHashRow> = rows.take(0)?;
    Ok(rows
        .into_iter()
        .map(|row| (record_key_to_string(row.id), row.hash))
        .collect())
}

pub async fn upsert_label_embeddings(records: &[LabelEmbeddingRecord]) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }

    DB.query(
        "FOR $record IN $records {
             UPSERT $record.id CONTENT $record RETURN NONE;
         };",
    )
    .bind(("records", records.to_vec()))
    .await?
    .check()?;

    Ok(())
}

pub async fn delete_label_embeddings_except(keys: &HashSet<String>) -> Result<()> {
    let ids: Vec<_> = keys
        .iter()
        .map(|key| RecordId::new("label_embeddings", key.as_str()))
        .collect();

    DB.query("DELETE label_embeddings WHERE id NOT IN $ids RETURN NONE")
        .bind(("ids", ids))
        .await?
        .check()?;

    Ok(())
}

pub async fn get_label_scores(embedding: &[f32]) -> Result<Vec<LabelScore>> {
    let mut res = DB
        .query(
            "
        SELECT label_group, label_value, vector::similarity::cosine(embedding, $embedding) AS similarity
        FROM label_embeddings
    ",
        )
        .bind(("embedding", embedding.to_vec()))
        .await?;

    Ok(res.take(0)?)
}

/// Returns the preference score for a given embedding, based on the users ratings.
pub async fn get_preference_score(embedding: &[f32]) -> Result<f32> {
    let mut res = DB
        .query(format!(
            "
        SELECT rating, vector::distance::knn() AS dist
        FROM user_articles
        WHERE rating != NONE AND embedding <|{PREFERENCE_NEIGHBOURS},{KNN_EF}|> $embedding
        ORDER BY dist
    "
        ))
        .bind(("embedding", embedding.to_vec()))
        .await?;

    let rows: Vec<PreferenceRow> = res.take(0)?;
    if rows.is_empty() {
        return Ok(0.5);
    }

    let (sum, weight) = rows.into_iter().fold((0.0, 0.0), |(sum, weight), row| {
        let sim = (1.0 - row.dist).clamp(-1.0, 1.0);
        let p = (sim * 10.0).exp();
        let val = match row.rating {
            Rating::Loved => 1.0,
            Rating::Liked => 0.75,
            Rating::Neutral => 0.5,
            Rating::Disliked => 0.25,
            Rating::Hated => 0.0,
        };
        (sum + val * p, weight + p)
    });

    Ok(if weight > 0.0 { sum / weight } else { 0.5 })
}

/// Retrieves all pending sources.
pub async fn get_pending_sources() -> Result<Vec<PendingSource>> {
    let sources: Vec<PendingSource> = DB.select("pending_sources").await?;
    Ok(sources)
}

/// Retrieves the count of articles per category that have a 'New' status.
pub async fn get_category_article_counts() -> Result<HashMap<Category, i64>> {
    let mut res = DB
        .query("SELECT category, count() FROM user_articles WHERE status = 'New' GROUP BY category")
        .await?;
    let rows: Vec<GroupedCount> = res.take(0)?;

    Ok(rows.into_iter().map(|r| (r.category, r.count)).collect())
}

/// Inserts a promoted article and removes its sources from pending.
pub async fn insert_promoted_article(article: Article, source_urls: Vec<String>) -> Result<()> {
    DB.query(
        "
        BEGIN TRANSACTION;
        UPSERT type::record('user_articles', $id) CONTENT $article RETURN NONE;
        DELETE pending_sources WHERE url IN $urls RETURN NONE;
        COMMIT TRANSACTION;
    ",
    )
    .bind(("id", article.id))
    .bind(("article", article))
    .bind(("urls", source_urls))
    .await?
    .check()?;

    Ok(())
}

pub async fn get_stale_embedding_records(
    table: &str,
    current_model: &str,
) -> Result<Vec<StaleEmbeddingRecord>> {
    if !matches!(table, "pending_sources" | "user_articles") {
        bail!("Unsupported embedding table: {table}");
    }

    let mut res = DB
        .query(format!(
            "SELECT id, embedding_text FROM {table}
             WHERE embedding = NONE OR embedding_model = NONE OR embedding_model != $model"
        ))
        .bind(("model", current_model))
        .await?;

    Ok(res.take(0)?)
}

pub async fn update_record_embeddings(
    updates: &[EmbeddingUpdate],
    embedding_model: &str,
) -> Result<()> {
    if updates.is_empty() {
        return Ok(());
    }

    DB.query(
        "FOR $record IN $updates {
             UPDATE $record.id MERGE {
                 embedding: $record.embedding,
                 embedding_model: $model
             } RETURN NONE;
         };",
    )
    .bind(("updates", updates.to_vec()))
    .bind(("model", embedding_model.to_string()))
    .await?
    .check()?;

    Ok(())
}

pub async fn purge_old_pending_sources() -> Result<()> {
    DB.query("DELETE pending_sources WHERE published < time::now() - 2w RETURN NONE")
        .await?
        .check()?;

    Ok(())
}

pub async fn get_similar_pending_sources(embedding: &[f32]) -> Result<Vec<(PendingSource, f32)>> {
    let mut res = DB
        .query(format!(
            "SELECT *, vector::distance::knn() AS dist
             FROM pending_sources
             WHERE embedding <|{SIMILAR_SOURCE_NEIGHBOURS},{KNN_EF}|> $embedding
             ORDER BY dist"
        ))
        .bind(("embedding", embedding.to_vec()))
        .await?;

    let rows: Vec<serde_json::Value> = res.take(0)?;
    let rows: Vec<SimilarSource> = rows
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<_, _>>()
        .context("Failed to decode a similar pending source row")?;
    Ok(rows.into_iter().map(|r| (r.source, 1.0 - r.dist)).collect())
}

/// Retrieves all generic item ratings.
pub async fn get_all_item_ratings() -> Result<HashMap<String, Rating>> {
    let mut res = DB.query("SELECT id, rating FROM item_ratings").await?;
    let rows: Vec<ItemRatingRow> = res.take(0)?;

    Ok(rows
        .into_iter()
        .map(|r| (record_key_to_string(r.id), r.rating))
        .collect())
}
