use crate::{
    server::embeddings::embedding_model_id,
    shared::{Article, ArticleStatus, Category, PendingSource, Rating},
};
use anyhow::{Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{
    QueryBuilder, Row, Sqlite, SqlitePool,
    sqlite::{
        SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow, SqliteSynchronous,
    },
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Once, OnceLock},
};
use uuid::Uuid;

static DB: OnceLock<SqlitePool> = OnceLock::new();
static REGISTER_SQLITE_VEC: Once = Once::new();

const DEFAULT_DATABASE_URL: &str = "sqlite://quivrs.db";
const PREFERENCE_NEIGHBOURS: usize = 64;
const SIMILAR_SOURCE_NEIGHBOURS: i64 = 30;

#[derive(Clone)]
pub struct LabelEmbeddingRecord {
    pub id: String,
    pub label_group: String,
    pub label_value: String,
    pub hash: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

pub struct LabelScore {
    pub label_group: String,
    pub label_value: String,
    pub similarity: f32,
}

#[derive(Clone, Copy)]
pub enum EmbeddingTable {
    PendingSources,
    UserArticles,
}

impl EmbeddingTable {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PendingSources => "pending_sources",
            Self::UserArticles => "user_articles",
        }
    }

    const fn stale_sql(self) -> &'static str {
        match self {
            Self::PendingSources => {
                "SELECT url AS id, embedding_text
                 FROM pending_sources
                 WHERE embedding = '[]' OR embedding_model = '' OR embedding_model != ?"
            }
            Self::UserArticles => {
                "SELECT id, embedding_text
                 FROM user_articles
                 WHERE embedding = '[]' OR embedding_model = '' OR embedding_model != ?"
            }
        }
    }

    const fn update_sql(self) -> &'static str {
        match self {
            Self::PendingSources => {
                "UPDATE pending_sources
                 SET embedding = ?, embedding_model = ?
                 WHERE url = ?"
            }
            Self::UserArticles => {
                "UPDATE user_articles
                 SET embedding = ?, embedding_model = ?
                 WHERE id = ?"
            }
        }
    }
}

fn pool() -> Result<&'static SqlitePool> {
    DB.get()
        .context("Database has not been initialised; call database::init first")
}

fn article_from_row(row: &SqliteRow) -> Result<Article> {
    let mut article: Article = serde_json::from_str(&row.get::<String, _>("payload"))?;
    article.status = row.get::<String, _>("status").parse()?;
    article.status_changed =
        DateTime::parse_from_rfc3339(&row.get::<String, _>("status_changed"))?.with_timezone(&Utc);
    article.rating = row
        .get::<Option<String>, _>("rating")
        .map(|rating| rating.parse())
        .transpose()?;
    article.embedding = serde_json::from_str(&row.get::<String, _>("embedding"))?;
    article.embedding_model = row.get("embedding_model");
    Ok(article)
}

fn pending_source_from_row(row: &SqliteRow) -> Result<PendingSource> {
    let mut source: PendingSource = serde_json::from_str(&row.get::<String, _>("payload"))?;
    source.embedding = serde_json::from_str(&row.get::<String, _>("embedding"))?;
    source.embedding_model = row.get("embedding_model");
    Ok(source)
}

pub async fn init() -> Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    if let Some(path) = database_url.strip_prefix("sqlite://")
        && path != ":memory:"
        && let Some(parent) = std::path::Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await?;
    }
    register_sqlite_vec();

    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!().run(&db).await?;
    DB.set(db).ok().context("Database already initialised")?;

    Ok(())
}

#[allow(unsafe_code)]
fn register_sqlite_vec() {
    REGISTER_SQLITE_VEC.call_once(|| unsafe {
        libsqlite3_sys::sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(
                *mut libsqlite3_sys::sqlite3,
                *mut *mut std::ffi::c_char,
                *const libsqlite3_sys::sqlite3_api_routines,
            ) -> i32,
        >(
            sqlite_vec::sqlite3_vec_init as *const ()
        )));
    });
}

/// Checks if a URL has already been processed, removes if they have.
pub async fn filter_new_urls(urls: &HashSet<String>) -> Result<Vec<String>> {
    if urls.is_empty() {
        return Ok(Vec::new());
    }

    let mut existing = HashSet::new();
    for chunk in urls.iter().collect::<Vec<_>>().chunks(256) {
        let mut query = QueryBuilder::new("SELECT url FROM article_urls WHERE url IN (");
        let mut values = query.separated(", ");
        for url in chunk {
            values.push_bind(*url);
        }
        values.push_unseparated(")");

        for row in query.build().fetch_all(pool()?).await? {
            existing.insert(row.get::<String, _>("url"));
        }
    }

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

    let mut query = QueryBuilder::new("INSERT OR IGNORE INTO article_urls (url) ");
    query.push_values(urls, |mut row, url| {
        row.push_bind(url);
    });
    query.build().execute(pool()?).await?;

    Ok(())
}

/// Stores newly found sources and records their URLs as processed.
pub async fn insert_sources(sources: Vec<PendingSource>) -> Result<()> {
    if sources.is_empty() {
        return Ok(());
    }

    let mut tx = pool()?.begin().await?;
    for source in sources {
        sqlx::query(
            "INSERT INTO pending_sources
                (url, payload, published, embedding, embedding_text, embedding_model)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(url) DO UPDATE SET
                payload = excluded.payload,
                published = excluded.published,
                embedding = excluded.embedding,
                embedding_text = excluded.embedding_text,
                embedding_model = excluded.embedding_model",
        )
        .bind(&source.url)
        .bind(serde_json::to_string(&source)?)
        .bind(source.published.to_rfc3339())
        .bind(serde_json::to_string(&source.embedding)?)
        .bind(&source.embedding_text)
        .bind(&source.embedding_model)
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO article_urls (url) VALUES (?)")
            .bind(&source.url)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub async fn set_article_status(id: Uuid, status: ArticleStatus) -> Result<()> {
    sqlx::query(
        "UPDATE user_articles
         SET status = ?, status_changed = ?
         WHERE id = ?",
    )
    .bind(status.to_string())
    .bind(Utc::now().to_rfc3339())
    .bind(id.to_string())
    .execute(pool()?)
    .await?;
    Ok(())
}

/// Sets a user rating for an article.
pub async fn set_rating(id: Uuid, rating: Rating) -> Result<()> {
    sqlx::query(
        "UPDATE user_articles
         SET rating = ?
         WHERE id = ?",
    )
    .bind(rating.to_string())
    .bind(id.to_string())
    .execute(pool()?)
    .await?;
    Ok(())
}

/// Retrieves all articles with their user-specific status and ratings.
pub async fn get_user_articles() -> Result<Vec<Article>> {
    let rows = sqlx::query(
        "SELECT payload, status, status_changed, rating, embedding, embedding_model
         FROM user_articles
         ORDER BY published DESC",
    )
    .fetch_all(pool()?)
    .await?;
    rows.iter().map(article_from_row).collect()
}

/// Sets a rating for a generic item (source, feed, or tag).
pub async fn set_item_rating(key: &str, rating: Rating) -> Result<()> {
    sqlx::query(
        "INSERT INTO item_ratings (id, rating) VALUES (?, ?)
         ON CONFLICT(id) DO UPDATE SET rating = excluded.rating",
    )
    .bind(key)
    .bind(rating.to_string())
    .execute(pool()?)
    .await?;
    Ok(())
}

pub async fn get_label_hashes() -> Result<HashMap<String, String>> {
    let rows = sqlx::query("SELECT id, hash FROM label_embeddings")
        .fetch_all(pool()?)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("id"), row.get("hash")))
        .collect())
}

pub async fn sync_label_embeddings(
    records: &[LabelEmbeddingRecord],
    keep_keys: &HashSet<String>,
) -> Result<()> {
    let mut tx = pool()?.begin().await?;
    for record in records {
        sqlx::query(
            "INSERT INTO label_embeddings
                (id, label_group, label_value, hash, text, embedding)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                label_group = excluded.label_group,
                label_value = excluded.label_value,
                hash = excluded.hash,
                text = excluded.text,
                embedding = excluded.embedding",
        )
        .bind(&record.id)
        .bind(&record.label_group)
        .bind(&record.label_value)
        .bind(&record.hash)
        .bind(&record.text)
        .bind(serde_json::to_string(&record.embedding)?)
        .execute(&mut *tx)
        .await?;
    }

    if keep_keys.is_empty() {
        sqlx::query("DELETE FROM label_embeddings")
            .execute(&mut *tx)
            .await?;
    } else {
        let mut query: QueryBuilder<Sqlite> =
            QueryBuilder::new("DELETE FROM label_embeddings WHERE id NOT IN (");
        let mut values = query.separated(", ");
        for key in keep_keys {
            values.push_bind(key);
        }
        values.push_unseparated(")");
        query.build().execute(&mut *tx).await?;
    }

    tx.commit().await?;

    Ok(())
}

pub async fn get_label_scores(embedding: &[f32]) -> Result<Vec<LabelScore>> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM label_embedding_vectors")
        .fetch_one(pool()?)
        .await?;
    if count == 0 {
        return Ok(Vec::new());
    }

    let rows = sqlx::query(
        "SELECT labels.label_group, labels.label_value, vectors.distance
         FROM label_embedding_vectors AS vectors
         JOIN label_embeddings AS labels ON labels.rowid = vectors.label_rowid
         WHERE vectors.embedding MATCH ? AND k = ?
         ORDER BY vectors.distance",
    )
    .bind(serde_json::to_string(embedding)?)
    .bind(count)
    .fetch_all(pool()?)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| LabelScore {
            label_group: row.get("label_group"),
            label_value: row.get("label_value"),
            similarity: 1.0 - row.get::<f32, _>("distance"),
        })
        .collect())
}

/// Returns the preference score for a given embedding, based on the users ratings.
pub async fn get_preference_score(embedding: &[f32]) -> Result<f32> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_article_embeddings")
        .fetch_one(pool()?)
        .await?;
    if count == 0 {
        return Ok(0.5);
    }

    let rows = sqlx::query(
        "SELECT articles.rating, vectors.distance
         FROM user_article_embeddings AS vectors
         JOIN user_articles AS articles ON articles.rowid = vectors.article_rowid
         WHERE vectors.embedding MATCH ? AND k = ? AND articles.rating IS NOT NULL
         ORDER BY vectors.distance",
    )
    .bind(serde_json::to_string(embedding)?)
    .bind(count)
    .fetch_all(pool()?)
    .await?;

    let rows = rows
        .into_iter()
        .map(|row| {
            Ok((
                row.get::<String, _>("rating").parse()?,
                1.0 - row.get::<f32, _>("distance"),
            ))
        })
        .take(PREFERENCE_NEIGHBOURS)
        .collect::<Result<Vec<_>>>()?;

    if rows.is_empty() {
        return Ok(0.5);
    }

    let (sum, weight) = rows.into_iter().fold((0.0, 0.0), |(sum, weight), row| {
        let sim = row.1.clamp(-1.0, 1.0);
        let p = (sim * 10.0).exp();
        let val = match row.0 {
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
    let rows = sqlx::query("SELECT payload, embedding, embedding_model FROM pending_sources")
        .fetch_all(pool()?)
        .await?;
    rows.iter().map(pending_source_from_row).collect()
}

/// Retrieves the count of articles per category that have a 'New' status.
pub async fn get_category_article_counts() -> Result<HashMap<Category, i64>> {
    let rows = sqlx::query(
        "SELECT category, COUNT(*) AS count
         FROM user_articles
         WHERE status = ?
         GROUP BY category",
    )
    .bind(ArticleStatus::New.to_string())
    .fetch_all(pool()?)
    .await?;

    rows.into_iter()
        .map(|row| {
            let category = row.get::<String, _>("category");
            Ok((
                category
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Unknown category value: {category}"))?,
                row.get::<i64, _>("count"),
            ))
        })
        .collect()
}

/// Inserts a promoted article and removes its sources from pending.
pub async fn insert_promoted_article(article: Article, source_urls: Vec<String>) -> Result<()> {
    let mut tx = pool()?.begin().await?;
    sqlx::query(
        "INSERT INTO user_articles
            (id, payload, published, category, status, status_changed, rating, embedding, embedding_text, embedding_model)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            payload = excluded.payload,
            published = excluded.published,
            category = excluded.category,
            status = excluded.status,
            status_changed = excluded.status_changed,
            rating = excluded.rating,
            embedding = excluded.embedding,
            embedding_text = excluded.embedding_text,
            embedding_model = excluded.embedding_model",
    )
    .bind(article.id.to_string())
    .bind(serde_json::to_string(&article)?)
    .bind(article.published.to_rfc3339())
    .bind(article.category.to_string())
    .bind(article.status.to_string())
    .bind(article.status_changed.to_rfc3339())
    .bind(article.rating.map(|r| r.to_string()))
    .bind(serde_json::to_string(&article.embedding)?)
    .bind(&article.embedding_text)
    .bind(&article.embedding_model)
    .execute(&mut *tx)
    .await?;

    if !source_urls.is_empty() {
        let mut query = QueryBuilder::new("DELETE FROM pending_sources WHERE url IN (");
        let mut values = query.separated(", ");
        for url in &source_urls {
            values.push_bind(url);
        }
        values.push_unseparated(")");
        query.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;

    Ok(())
}

pub async fn get_stale_embedding_records(table: EmbeddingTable) -> Result<Vec<(String, String)>> {
    let rows = sqlx::query(table.stale_sql())
        .bind(embedding_model_id())
        .fetch_all(pool()?)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("id"), row.get("embedding_text")))
        .collect())
}

pub async fn update_record_embeddings(
    table: EmbeddingTable,
    updates: &[(String, Vec<f32>)],
) -> Result<()> {
    if updates.is_empty() {
        return Ok(());
    }

    let mut tx = pool()?.begin().await?;
    for (id, embedding) in updates {
        let embedding_json = serde_json::to_string(embedding)?;
        sqlx::query(table.update_sql())
            .bind(&embedding_json)
            .bind(embedding_model_id())
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(())
}

pub async fn purge_old_pending_sources() -> Result<()> {
    let cutoff = Utc::now() - TimeDelta::weeks(2);
    sqlx::query("DELETE FROM pending_sources WHERE published < ?")
        .bind(cutoff.to_rfc3339())
        .execute(pool()?)
        .await?;

    Ok(())
}

pub async fn get_similar_pending_sources(embedding: &[f32]) -> Result<Vec<(PendingSource, f32)>> {
    let rows = sqlx::query(
        "SELECT sources.payload, sources.embedding, sources.embedding_model, vectors.distance
         FROM pending_source_embeddings AS vectors
         JOIN pending_sources AS sources ON sources.rowid = vectors.source_rowid
         WHERE vectors.embedding MATCH ? AND k = ?
         ORDER BY vectors.distance",
    )
    .bind(serde_json::to_string(embedding)?)
    .bind(SIMILAR_SOURCE_NEIGHBOURS)
    .fetch_all(pool()?)
    .await?;

    rows.into_iter()
        .map(|row| {
            let source = pending_source_from_row(&row)?;
            Ok((source, 1.0 - row.get::<f32, _>("distance")))
        })
        .collect()
}

/// Retrieves all generic item ratings.
pub async fn get_all_item_ratings() -> Result<HashMap<String, Rating>> {
    let rows = sqlx::query("SELECT id, rating FROM item_ratings")
        .fetch_all(pool()?)
        .await?;

    rows.into_iter()
        .map(|row| {
            Ok((
                row.get::<String, _>("id"),
                row.get::<String, _>("rating").parse()?,
            ))
        })
        .collect()
}
