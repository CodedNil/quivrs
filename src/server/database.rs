use crate::shared::{ArticleEntry, ArticleSource, StoredArticle};
use anyhow::{Result, anyhow};
use chrono::DateTime;
use rusqlite::{Connection, params};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    env,
    sync::{LazyLock, Mutex},
};
use uuid::Uuid;

static DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.db".to_string());
    let conn = Connection::open(&url).expect("Failed to open database");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS articles (
             id              TEXT PRIMARY KEY,
             title           TEXT NOT NULL,
             sources         BLOB NOT NULL,
             sources_count   INTEGER NOT NULL DEFAULT 1,
             estimated_liked REAL NOT NULL DEFAULT 0.0,
             entry           BLOB,
             embedding       BLOB NOT NULL,
             published       INTEGER NOT NULL,
             updated_at      INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS published
             ON articles(published);
         CREATE TABLE IF NOT EXISTS article_urls (
             url        TEXT PRIMARY KEY,
             article_id TEXT NOT NULL
         );",
    )
    .expect("Failed to initialize database schema");
    Mutex::new(conn)
});

fn encode<T: Serialize + ?Sized>(val: &T) -> Result<Vec<u8>> {
    postcard::to_allocvec(val).map_err(|e| anyhow!("{e}"))
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    postcard::from_bytes(bytes).map_err(|e| anyhow!("{e}"))
}

pub struct EmbeddingCandidate {
    pub id: Uuid,
    pub title: String,
    pub embedding: Vec<f32>,
}

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

pub fn get_embedding_candidates(
    window_start: i64,
    window_end: i64,
) -> Result<Vec<EmbeddingCandidate>> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        "SELECT id, title, embedding FROM articles WHERE published BETWEEN ?1 AND ?2",
    )?;
    let candidates = stmt
        .query_map(params![window_start, window_end], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })?
        .filter_map(Result::ok)
        .filter_map(|(id_str, title, emb_bytes)| {
            Some(EmbeddingCandidate {
                id: Uuid::parse_str(&id_str).ok()?,
                title,
                embedding: decode(&emb_bytes).ok()?,
            })
        })
        .collect();
    drop(stmt);
    drop(conn);
    Ok(candidates)
}

pub fn insert_article(source: &ArticleSource, embedding: &[f32]) -> Result<()> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp();
    let url = source.url.clone();
    let title = source.title.clone();
    let published = source.published.timestamp();
    let sources_bytes = encode(std::slice::from_ref(&source))?;
    let emb_bytes = encode(embedding)?;

    let conn = DB.lock().unwrap();
    conn.execute(
        "INSERT INTO articles
         (id, title, sources, sources_count, estimated_liked, entry, embedding, published, updated_at)
         VALUES (?1, ?2, ?3, 1, 0.0, NULL, ?4, ?5, ?6)",
        params![id.to_string(), title, sources_bytes, emb_bytes, published, now],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO article_urls (url, article_id) VALUES (?1, ?2)",
        params![url, id.to_string()],
    )?;
    drop(conn);
    Ok(())
}

pub fn merge_into_article(
    article_id: Uuid,
    source: ArticleSource,
    embedding: &[f32],
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let url = source.url.clone();

    let conn = DB.lock().unwrap();
    let (sources_bytes, existing_emb_bytes): (Vec<u8>, Vec<u8>) = conn.query_row(
        "SELECT sources, embedding FROM articles WHERE id = ?1",
        params![article_id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut sources: Vec<ArticleSource> = decode(&sources_bytes)?;
    sources.push(source);
    let sources_count = sources.len() as i64;
    let sources_bytes = encode(&sources)?;

    let existing_emb: Vec<f32> = decode(&existing_emb_bytes)?;
    let merged_emb: Vec<f32> = existing_emb
        .iter()
        .zip(embedding.iter())
        .map(|(a, b)| (a + b) / 2.0)
        .collect();
    let emb_bytes = encode(merged_emb.as_slice())?;

    conn.execute(
        "UPDATE articles SET sources = ?1, sources_count = ?2, embedding = ?3, entry = NULL, updated_at = ?4
         WHERE id = ?5",
        params![sources_bytes, sources_count, emb_bytes, now, article_id.to_string()],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO article_urls (url, article_id) VALUES (?1, ?2)",
        params![url, article_id.to_string()],
    )?;
    drop(conn);
    Ok(())
}

pub fn get_regeneration_targets() -> Result<Vec<(Uuid, Vec<ArticleSource>)>> {
    let conn = DB.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT id, sources FROM articles WHERE entry IS NULL AND sources_count > 1")?;
    let targets = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?
        .filter_map(Result::ok)
        .filter_map(|(id_str, sources_bytes)| {
            Some((
                Uuid::parse_str(&id_str).ok()?,
                decode::<Vec<ArticleSource>>(&sources_bytes).ok()?,
            ))
        })
        .collect();
    drop(stmt);
    drop(conn);
    Ok(targets)
}

pub fn save_article_entry(article_id: Uuid, entry: &ArticleEntry) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let entry_bytes = encode(entry)?;
    let conn = DB.lock().unwrap();
    conn.execute(
        "UPDATE articles SET entry = ?1, updated_at = ?2 WHERE id = ?3",
        params![entry_bytes, now, article_id.to_string()],
    )?;
    drop(conn);
    Ok(())
}

pub fn get_all_articles() -> Result<Vec<StoredArticle>> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, sources, estimated_liked, entry, embedding, published, updated_at
         FROM articles ORDER BY updated_at DESC",
    )?;
    let articles = stmt
        .query_and_then([], |row| -> Result<StoredArticle> {
            Ok(StoredArticle {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| anyhow!("{e}"))?,
                sources: decode(&row.get::<_, Vec<u8>>(1)?)?,
                #[allow(clippy::cast_possible_truncation)]
                estimated_liked: row.get::<_, f64>(2)? as f32,
                entry: row
                    .get::<_, Option<Vec<u8>>>(3)?
                    .and_then(|b| decode(&b).ok()),
                embedding: decode(&row.get::<_, Vec<u8>>(4)?)?,
                published: DateTime::from_timestamp(row.get::<_, i64>(5)?, 0).unwrap_or_default(),
                updated_at: DateTime::from_timestamp(row.get::<_, i64>(6)?, 0).unwrap_or_default(),
            })
        })?
        .filter_map(Result::ok)
        .collect();
    drop(stmt);
    drop(conn);
    Ok(articles)
}
