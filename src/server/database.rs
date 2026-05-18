use crate::{
    server::embeddings::{MODEL_NAME, generate_embeddings},
    shared::{
        ArticleData, ArticleEntry, ArticleSource, ArticleStatus, ArticleType, Category, Rating,
        StoredArticle,
    },
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use postcard::{from_bytes, to_allocvec};
use rusqlite::{Connection, params};
use std::{
    collections::HashMap,
    env,
    sync::{LazyLock, Mutex},
};
use tracing::{error, info};
use uuid::Uuid;

/// Global database connection pool
static DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.db".to_string());
    let conn = Connection::open(&url).expect("Failed to open database");

    // Initialize database schema.
    // WAL mode allows concurrent readers while a writer is active.
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;

         -- Core article data
         CREATE TABLE IF NOT EXISTS articles (
             id               TEXT PRIMARY KEY,
             title            TEXT NOT NULL,
             sources          BLOB NOT NULL,
             estimated_liked  REAL NOT NULL DEFAULT 0.0,
             entry            BLOB,
             embedding        BLOB NOT NULL,
             embedding_model  TEXT NOT NULL DEFAULT '',
             article_type     TEXT NOT NULL DEFAULT 'News',
             category         TEXT NOT NULL DEFAULT 'Technology',
             published        INTEGER NOT NULL,
             updated_at       INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS published ON articles(published);

         -- Mapping of individual URLs to article IDs
         CREATE TABLE IF NOT EXISTS article_urls (
             url        TEXT PRIMARY KEY,
             article_id TEXT NOT NULL
         );

         -- User-specific state for articles
         CREATE TABLE IF NOT EXISTS user_articles (
             article_id TEXT PRIMARY KEY,
             status     TEXT NOT NULL DEFAULT 'New',
             binned_at  INTEGER
         );

         -- User ratings for specific articles
         CREATE TABLE IF NOT EXISTS article_ratings (
             article_id TEXT PRIMARY KEY,
             rating     TEXT NOT NULL
         );

         -- Generic item ratings
         CREATE TABLE IF NOT EXISTS item_ratings (
             key    TEXT PRIMARY KEY,
             rating TEXT NOT NULL
         );",
    )
    .expect("Failed to initialize database schema");

    Mutex::new(conn)
});

pub struct EmbeddingCandidate {
    pub id: Uuid,
    pub title: String,
    pub embedding: Vec<f32>,
}

/// Checks if a URL has already been processed.
pub fn url_exists(url: &str) -> bool {
    DB.lock()
        .unwrap()
        .query_row(
            "SELECT 1 FROM article_urls WHERE url = ?1",
            params![url],
            |_| Ok(()),
        )
        .is_ok()
}

/// Retrieves articles within a time window for similarity comparison.
pub fn get_embedding_candidates(
    window_start: i64,
    window_end: i64,
) -> Result<Vec<EmbeddingCandidate>> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        "SELECT id, title, embedding FROM articles WHERE published BETWEEN ? AND ? AND embedding_model = ?",
    )?;

    let candidates = stmt
        .query_and_then(
            params![window_start, window_end, MODEL_NAME],
            |row| -> Result<EmbeddingCandidate> {
                let id_str: String = row.get(0)?;
                let title: String = row.get(1)?;
                let emb_bytes: Vec<u8> = row.get(2)?;
                Ok(EmbeddingCandidate {
                    id: Uuid::parse_str(&id_str)?,
                    title,
                    embedding: from_bytes(&emb_bytes)?,
                })
            },
        )?
        .collect::<Result<Vec<_>>>()?;

    Ok(candidates)
}

/// Inserts a new article and its initial metadata.
pub fn insert_article(
    source: &ArticleSource,
    embedding: &[f32],
    article_type: ArticleType,
    category: Category,
) -> Result<()> {
    let id_str = Uuid::new_v4().to_string();
    let mut conn = DB.lock().unwrap();
    let tx = conn.transaction()?;

    // Insert the master article track
    tx.execute(
        "INSERT INTO articles (id, title, sources, estimated_liked, entry, embedding, embedding_model, published, updated_at, article_type, category)
        VALUES (?, ?, ?, 0.0, NULL, ?, ?, ?, ?, ?, ?)",
        params![
            id_str,
            source.title,
            to_allocvec(std::slice::from_ref(source))?,
            to_allocvec(embedding)?,
            MODEL_NAME,
            source.published.timestamp(),
            Utc::now().timestamp(),
            article_type.to_string(),
            category.to_string(),
        ],
    )?;

    // Map url tracking index
    tx.execute(
        "INSERT INTO article_urls (url, article_id) VALUES (?, ?)",
        params![source.url, id_str],
    )?;

    // Initialize user view state link
    tx.execute(
        "INSERT INTO user_articles (article_id, status) VALUES (?, 'New')",
        params![id_str],
    )?;

    tx.commit()?;
    Ok(())
}

/// Merges a new source into an existing article.
pub fn merge_into_article(
    article_id: Uuid,
    source: &ArticleSource,
    embedding: &[f32],
) -> Result<()> {
    let id_str = article_id.to_string();
    let mut conn = DB.lock().unwrap();
    let tx = conn.transaction()?;

    // Fetch current data
    let (sources_bytes, existing_emb_bytes): (Vec<u8>, Vec<u8>) = tx.query_row(
        "SELECT sources, embedding FROM articles WHERE id = ?",
        params![id_str],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // Append new source
    let mut sources: Vec<ArticleSource> = from_bytes(&sources_bytes)?;
    sources.push(source.clone());

    // Merge embeddings
    let existing_emb: Vec<f32> = from_bytes(&existing_emb_bytes)?;
    let merged_emb: Vec<f32> = existing_emb
        .iter()
        .zip(embedding)
        .map(|(a, b)| (a + b) / 2.0)
        .collect();

    // Update article data
    tx.execute(
        "UPDATE articles SET sources = ?, embedding = ?, embedding_model = ?, entry = NULL, updated_at = ? WHERE id = ?",
        params![
            to_allocvec(&sources)?,
            to_allocvec(&merged_emb)?,
            MODEL_NAME,
            Utc::now().timestamp(),
            id_str
        ],
    )?;

    // Map the new URL
    tx.execute(
        "INSERT OR IGNORE INTO article_urls (url, article_id) VALUES (?, ?)",
        params![source.url, id_str],
    )?;

    tx.commit()?;
    Ok(())
}

/// Embeddings generated with an older model will be regenerated.
pub async fn regenerate_stale_embeddings() -> Result<()> {
    let stale = {
        let conn = DB.lock().unwrap();
        conn.prepare_cached("SELECT id, sources FROM articles WHERE embedding_model != ?")?
            .query_and_then(
                params![MODEL_NAME],
                |row| -> Result<(Uuid, Vec<ArticleSource>)> {
                    let id_str: String = row.get(0)?;
                    let sources_bytes: Vec<u8> = row.get(1)?;
                    Ok((Uuid::parse_str(&id_str)?, from_bytes(&sources_bytes)?))
                },
            )?
            .collect::<Result<Vec<_>>>()
    }?;
    if stale.is_empty() {
        return Ok(());
    }

    info!("Re-embedding {} articles for new model...", stale.len());

    let texts: Vec<String> = stale
        .iter()
        .map(|(_, sources)| {
            sources
                .iter()
                .map(|s| format!("{} {} {}", s.url, s.title, s.summary))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();

    let embeddings = generate_embeddings(&texts)
        .await
        .inspect_err(|e| error!("Stale embedding regeneration failed: {e}"))?;

    for ((id, _), embedding) in stale.iter().zip(embeddings) {
        DB.lock().unwrap().execute(
            "UPDATE articles SET embedding = ?, embedding_model = ?, updated_at = ? WHERE id = ?",
            params![
                to_allocvec(&embedding)?,
                MODEL_NAME,
                Utc::now().timestamp(),
                id.to_string(),
            ],
        )?;
    }

    Ok(())
}

/// Finds articles that have multiple sources but haven't had a merged entry generated yet.
pub fn get_regeneration_targets() -> Result<Vec<(Uuid, Vec<ArticleSource>)>> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare_cached("SELECT id, sources FROM articles WHERE entry IS NULL")?;

    let targets = stmt
        .query_and_then([], |row| -> Result<(Uuid, Vec<ArticleSource>)> {
            let id_str: String = row.get(0)?;
            let sources_bytes: Vec<u8> = row.get(1)?;

            Ok((Uuid::parse_str(&id_str)?, from_bytes(&sources_bytes)?))
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(targets)
}

/// Saves the LLM-generated summary/entry for an article.
pub fn save_article_entry(article_id: Uuid, entry: &ArticleEntry) -> Result<()> {
    DB.lock().unwrap().execute(
        "UPDATE articles SET entry = ?, updated_at = ? WHERE id = ?",
        params![
            to_allocvec(entry)?,
            Utc::now().timestamp(),
            article_id.to_string()
        ],
    )?;
    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub fn set_article_status(article_id: Uuid, status: ArticleStatus) -> Result<()> {
    let binned_at = (status == ArticleStatus::Binned).then(|| Utc::now().timestamp());
    DB.lock().unwrap().execute(
        "INSERT INTO user_articles (article_id, status, binned_at) VALUES (?, ?, ?)
         ON CONFLICT(article_id) DO UPDATE SET status = excluded.status, binned_at = excluded.binned_at",
        params![article_id.to_string(), status.to_string(), binned_at],
    )?;
    Ok(())
}

/// Sets a user rating (Like/Dislike) for an article.
pub fn set_rating(article_id: Uuid, rating: Rating) -> Result<()> {
    DB.lock().unwrap().execute(
        "INSERT INTO article_ratings (article_id, rating) VALUES (?, ?)
         ON CONFLICT(article_id) DO UPDATE SET rating = excluded.rating",
        params![article_id.to_string(), rating.to_string()],
    )?;
    Ok(())
}

/// Retrieves all articles with their user-specific status and ratings.
pub fn get_user_articles() -> Result<Vec<ArticleData>> {
    let conn = DB.lock().unwrap();
    conn.prepare_cached(
        "SELECT ua.article_id, ua.status, ar.rating, a.sources, a.estimated_liked, a.entry, a.published, a.updated_at, a.article_type, a.category
         FROM user_articles ua
         JOIN articles a ON ua.article_id = a.id
         LEFT JOIN article_ratings ar ON ua.article_id = ar.article_id
         ORDER BY a.published DESC",
    )?
    .query_and_then([], |row| -> Result<ArticleData> {
        let id = Uuid::parse_str(&row.get::<_, String>(0)?)?;
        let status = row
            .get::<_, String>(1)?
            .parse()
            .map_err(|e| anyhow!("{e}"))?;
        let rating = row
            .get::<_, Option<String>>(2)?
            .map(|s| s.parse())
            .transpose()
            .map_err(|e| anyhow!("{e}"))?;
        let article_type = row
            .get::<_, String>(8)?
            .parse()
            .unwrap_or(ArticleType::News);
        let category = row
            .get::<_, String>(9)?
            .parse()
            .unwrap_or(Category::Technology);

        let art = StoredArticle {
            id,
            sources: from_bytes(&row.get::<_, Vec<u8>>(3)?)?,
            estimated_liked: row.get::<_, f64>(4)? as f32,
            entry: row
                .get::<_, Option<Vec<u8>>>(5)?
                .map(|b| from_bytes(&b))
                .transpose().unwrap_or_default(),
            published: DateTime::from_timestamp(row.get::<_, i64>(6)?, 0).unwrap_or_default(),
            updated_at: DateTime::from_timestamp(row.get::<_, i64>(7)?, 0).unwrap_or_default(),
            article_type,
            category,
        };
        Ok((id, status, rating, art))
    })?
    .collect::<Result<Vec<_>>>()
}

/// Deletes articles from `user_articles` that have been binned for longer than the specified days.
pub fn cleanup_binned(days: i64) -> Result<()> {
    let cutoff = Utc::now().timestamp() - days * 86_400;
    DB.lock().unwrap().execute(
        "DELETE FROM user_articles WHERE status = 'Binned' AND binned_at < ?",
        params![cutoff],
    )?;
    Ok(())
}

/// Sets a rating for a generic item (source, feed, or tag).
pub fn set_item_rating(key: &str, rating: Rating) -> Result<()> {
    DB.lock().unwrap().execute(
        "INSERT INTO item_ratings (key, rating) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET rating = excluded.rating",
        params![key, rating.to_string()],
    )?;
    Ok(())
}

/// Retrieves all generic item ratings.
pub fn get_all_item_ratings() -> Result<HashMap<String, Rating>> {
    let conn = DB.lock().unwrap();
    conn.prepare_cached("SELECT key, rating FROM item_ratings")?
        .query_and_then([], |row| -> Result<(String, Rating)> {
            let key: String = row.get(0)?;
            let rating: Rating = row
                .get::<_, String>(1)?
                .parse()
                .map_err(|e| anyhow!("{e}"))?;
            Ok((key, rating))
        })?
        .collect::<Result<_>>()
}
