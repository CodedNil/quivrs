use crate::shared::{Article, ArticleStatus, Category, PendingSource, Rating};
use anyhow::{Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};
use surrealdb::Surreal;
use surrealdb::engine::local::SurrealKv;
use surrealdb::types::{RecordId as Thing, SurrealValue};
use tracing::{info, warn};
use uuid::Uuid;

pub static DB: LazyLock<Surreal<surrealdb::engine::local::Db>> = LazyLock::new(Surreal::init);

const PREFERENCE_NEIGHBOURS: u32 = 64;
const SIMILAR_SOURCE_NEIGHBOURS: u32 = 50;
const KNN_EF: u32 = 200;

#[derive(Serialize, Deserialize, SurrealValue)]
struct ArticleUrl {
    id: Thing,
}

#[derive(Serialize, Deserialize, SurrealValue)]
struct ItemRatingRecord {
    rating: Rating,
}

#[derive(Clone, Serialize, Deserialize, SurrealValue)]
pub struct LabelEmbeddingRecord {
    pub label_group: String,
    pub label_value: String,
    pub hash: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

#[derive(Deserialize, SurrealValue)]
struct LabelHashRow {
    id: Thing,
    hash: String,
}

#[derive(Deserialize, SurrealValue)]
pub struct LabelScore {
    pub label_value: String,
    pub similarity: f32,
}

#[derive(Deserialize, SurrealValue)]
struct GroupedCount {
    category: Category,
    count: i64,
}

#[derive(Deserialize, SurrealValue)]
pub struct StaleEmbeddingRecord {
    pub id: Thing,
    pub embedding_text: String,
}

#[derive(Deserialize)]
struct SimilarSource {
    #[serde(flatten)]
    source: PendingSource,
    dist: f32,
}

#[derive(Deserialize, SurrealValue)]
struct PreferenceRow {
    rating: Rating,
    dist: f32,
}

#[derive(Deserialize, SurrealValue)]
struct ItemRatingRow {
    id: Thing,
    rating: Rating,
}

fn record_key_to_string(id: Thing) -> String {
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
        return Ok(vec![]);
    }

    let mut res = DB
        .query("SELECT VALUE id FROM article_urls WHERE id IN $urls")
        .bind((
            "urls",
            urls.iter()
                .map(|u| Thing::new("article_urls", u.as_str()))
                .collect::<Vec<_>>(),
        ))
        .await?;

    let existing_ids: Vec<Thing> = res.take(0)?;
    let existing: HashSet<String> = existing_ids.into_iter().map(record_key_to_string).collect();

    Ok(urls
        .iter()
        .filter(|u| !existing.contains(*u))
        .cloned()
        .collect())
}

/// Records URLs as permanently skipped so they are never re-fetched.
pub async fn mark_urls_dismissed(urls: &[String]) -> Result<()> {
    for url in urls {
        let _: Option<ArticleUrl> = DB
            .upsert::<Option<ArticleUrl>>(("article_urls", url.as_str()))
            .content(ArticleUrl {
                id: Thing::new("article_urls", url.as_str()),
            })
            .await?;
    }
    Ok(())
}

/// Inserts a newly found source.
pub async fn insert_source(source: &PendingSource) -> Result<()> {
    let res = DB
        .query(
            "BEGIN TRANSACTION;
              CREATE pending_sources CONTENT $source;
              UPSERT type::record('article_urls', $url) CONTENT { id: type::record('article_urls', $url) };
              COMMIT TRANSACTION;",
        )
        .bind(("source", source.clone()))
        .bind(("url", source.url.clone()))
        .await?;

    if let Err(e) = res.check() {
        // If it's a unique constraint error on the URL, we just ignore it as it's already in the DB
        let err_str = e.to_string();
        if !err_str.contains("Database index `source_url` already contains") {
            warn!("Failed to insert source '{}': {}", source.title, e);
        }
    }

    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub async fn set_article_status(id: Uuid, status: ArticleStatus) -> Result<()> {
    let binned_at = (status == ArticleStatus::Binned).then(Utc::now);
    DB.query(
        "UPDATE type::record('user_articles', $id)
         SET status = $status, binned_at = $binned_at",
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
    DB.query("UPDATE type::record('user_articles', $id) SET rating = $rating")
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
    DB.upsert::<Option<ItemRatingRecord>>(("item_ratings", key))
        .content(ItemRatingRecord { rating })
        .await?;
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

pub async fn upsert_label_embeddings(records: &[(String, LabelEmbeddingRecord)]) -> Result<()> {
    for (key, record) in records {
        let _: Option<LabelEmbeddingRecord> = DB
            .upsert(Thing::new("label_embeddings", key.as_str()))
            .content(record.clone())
            .await?;
    }

    Ok(())
}

pub async fn delete_label_embeddings_except(keys: &HashSet<String>) -> Result<()> {
    let ids: Vec<_> = keys
        .iter()
        .map(|key| Thing::new("label_embeddings", key.as_str()))
        .collect();

    DB.query("DELETE label_embeddings WHERE id NOT IN $ids")
        .bind(("ids", ids))
        .await?
        .check()?;

    Ok(())
}

pub async fn get_label_scores(label_group: &str, embedding: &[f32]) -> Result<Vec<LabelScore>> {
    let mut res = DB
        .query(
            "
        SELECT label_value, vector::similarity::cosine(embedding, $embedding) AS similarity
        FROM label_embeddings
        WHERE label_group = $label_group
        ORDER BY similarity DESC
    ",
        )
        .bind(("label_group", label_group.to_string()))
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
pub async fn insert_promoted_article(article: &Article, source_urls: &[String]) -> Result<()> {
    DB.query(
        "
        BEGIN TRANSACTION;
        UPSERT type::record('user_articles', $id) CONTENT $article;
        DELETE pending_sources WHERE url IN $urls;
        COMMIT TRANSACTION;
    ",
    )
    .bind(("id", article.id))
    .bind(("article", article.clone()))
    .bind(("urls", source_urls.to_vec()))
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
            "SELECT id, embedding_text FROM {table} WHERE embedding_model != $model"
        ))
        .bind(("model", current_model))
        .await?;

    Ok(res.take(0)?)
}

pub async fn update_record_embedding(
    id: Thing,
    embedding: Vec<f32>,
    embedding_model: &str,
) -> Result<()> {
    DB.update::<Option<surrealdb::types::Value>>(id)
        .merge(serde_json::json!({
            "embedding": embedding,
            "embedding_model": embedding_model
        }))
        .await?;

    Ok(())
}

pub async fn purge_old_pending_sources() -> Result<()> {
    DB.query("DELETE pending_sources WHERE published < time::now() - 2w")
        .await?
        .check()?;

    Ok(())
}

pub async fn get_similar_pending_sources(
    embedding: &[f32],
    category: Category,
    min_published: chrono::DateTime<chrono::Utc>,
    max_published: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<(PendingSource, f32)>> {
    let mut res = DB
        .query(format!(
            "SELECT *, vector::distance::knn() AS dist
             FROM pending_sources
             WHERE category = $category
               AND published > $min
               AND published < $max
               AND embedding <|{SIMILAR_SOURCE_NEIGHBOURS},{KNN_EF}|> $embedding
             ORDER BY dist"
        ))
        .bind(("embedding", embedding.to_vec()))
        .bind(("category", category))
        .bind(("min", min_published))
        .bind(("max", max_published))
        .await?;

    let rows: Vec<serde_json::Value> = res.take(0)?;
    let rows: Vec<SimilarSource> = rows
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<_, _>>()?;
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
