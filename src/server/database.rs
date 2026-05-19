use crate::server::embeddings::{MODEL_NAME, generate_article_embeddings};
use crate::shared::{
    ArticleData, ArticleEntry, ArticleSource, ArticleStatus, ArticleType, Category, Rating,
    StoredArticle,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use postcard::{from_bytes, to_allocvec};
use sqlx::{
    QueryBuilder, Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    collections::{HashMap, HashSet},
    env,
    str::FromStr,
    sync::LazyLock,
};
use tracing::{error, info};
use uuid::Uuid;

static DB: LazyLock<SqlitePool> = LazyLock::new(|| {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:quivrs.db".to_string());
    let opts = SqliteConnectOptions::from_str(&url)
        .expect("Invalid DATABASE_URL")
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .create_if_missing(true);
    SqlitePoolOptions::new().connect_lazy_with(opts)
});

pub async fn init() -> Result<()> {
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS articles (
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
         CREATE TABLE IF NOT EXISTS article_urls (
             url        TEXT PRIMARY KEY,
             article_id TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS user_articles (
             article_id TEXT PRIMARY KEY,
             status     TEXT NOT NULL DEFAULT 'New',
             binned_at  INTEGER
         );
         CREATE TABLE IF NOT EXISTS article_ratings (
             article_id TEXT PRIMARY KEY,
             rating     TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS item_ratings (
             key    TEXT PRIMARY KEY,
             rating TEXT NOT NULL
         );",
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

pub struct EmbeddingCandidate {
    pub id: Uuid,
    pub title: String,
    pub embedding: Vec<f32>,
}

/// Checks if a URL has already been processed, removes if they have.
pub async fn filter_new_urls(urls: &HashSet<String>) -> Result<Vec<String>> {
    if urls.is_empty() {
        return Ok(vec![]);
    }
    let mut builder = QueryBuilder::new("SELECT url FROM article_urls WHERE url IN (");
    let mut separated = builder.separated(", ");
    for url in urls {
        separated.push_bind(url);
    }
    separated.push_unseparated(")");
    let existing: HashSet<String> = builder
        .build()
        .fetch_all(&*DB)
        .await?
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    Ok(urls.difference(&existing).cloned().collect())
}

/// Records URLs as permanently skipped so they are never re-fetched.
pub async fn mark_urls_dismissed(urls: &[String]) -> Result<()> {
    if urls.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::new("INSERT OR IGNORE INTO article_urls (url, article_id) ");
    builder.push_values(urls.iter(), |mut b, url| {
        b.push_bind(url).push_bind(Uuid::nil());
    });
    builder.build().execute(&*DB).await?;
    Ok(())
}

/// Retrieves articles within a time window for similarity comparison.
pub async fn get_embedding_candidates(
    window_start: i64,
    window_end: i64,
) -> Result<Vec<EmbeddingCandidate>> {
    sqlx::query_as::<_, (Uuid, String, Vec<u8>)>(
        "SELECT id, title, embedding FROM articles
         WHERE published BETWEEN ? AND ? AND embedding_model = ?",
    )
    .bind(window_start)
    .bind(window_end)
    .bind(MODEL_NAME)
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|(id, title, embedding)| {
        Ok(EmbeddingCandidate {
            id,
            title,
            embedding: from_bytes(&embedding)?,
        })
    })
    .collect()
}

/// Inserts a new article and its initial metadata.
pub async fn insert_article(
    source: &ArticleSource,
    embedding: &[f32],
    article_type: ArticleType,
    category: Category,
) -> Result<()> {
    let id = Uuid::new_v4();
    let sources_bytes = to_allocvec(std::slice::from_ref(source))?;
    let embedding_bytes = to_allocvec(embedding)?;

    let mut tx = DB.begin().await?;
    sqlx::query(
        "INSERT INTO articles
             (id, title, sources, estimated_liked, entry, embedding, embedding_model,
              published, updated_at, article_type, category)
         VALUES (?, ?, ?, 0.0, NULL, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&source.title)
    .bind(&sources_bytes)
    .bind(&embedding_bytes)
    .bind(MODEL_NAME)
    .bind(source.published.timestamp())
    .bind(Utc::now().timestamp())
    .bind(article_type.to_string())
    .bind(category.to_string())
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO article_urls (url, article_id) VALUES (?, ?)")
        .bind(&source.url)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO user_articles (article_id, status) VALUES (?, 'New')")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Merges a new source into an existing article.
pub async fn merge_into_article(
    article_id: Uuid,
    source: &ArticleSource,
    embedding: &[f32],
) -> Result<()> {
    let mut tx = DB.begin().await?;

    let (sources_bytes, existing_emb_bytes): (Vec<u8>, Vec<u8>) =
        sqlx::query_as("SELECT sources, embedding FROM articles WHERE id = ?")
            .bind(article_id)
            .fetch_one(&mut *tx)
            .await?;

    let mut sources: Vec<ArticleSource> = from_bytes(&sources_bytes)?;
    sources.push(source.clone());

    let existing_emb: Vec<f32> = from_bytes(&existing_emb_bytes)?;
    let merged_emb: Vec<f32> = existing_emb
        .iter()
        .zip(embedding)
        .map(|(a, b)| (a + b) / 2.0)
        .collect();

    sqlx::query(
        "UPDATE articles
         SET sources = ?, embedding = ?, embedding_model = ?, entry = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(to_allocvec(&sources)?)
    .bind(to_allocvec(&merged_emb)?)
    .bind(MODEL_NAME)
    .bind(Utc::now().timestamp())
    .bind(article_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT OR IGNORE INTO article_urls (url, article_id) VALUES (?, ?)")
        .bind(&source.url)
        .bind(article_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Rerun the classification on articles
pub async fn reclassify_articles(ids: Vec<Uuid>) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    let mut builder =
        QueryBuilder::new("SELECT id, sources, article_type, category FROM articles WHERE id IN (");
    let mut separated = builder.separated(", ");
    for id in &ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(")");

    let targets: Vec<(Uuid, Vec<u8>, String, String)> = builder
        .build_query_as::<(Uuid, Vec<u8>, String, String)>()
        .fetch_all(&*DB)
        .await?;

    if targets.is_empty() {
        return Ok(());
    }

    info!("Re-classifying {} articles...", targets.len());

    let first_sources: Vec<ArticleSource> = targets
        .iter()
        .map(|(_, sources_bytes, _, _)| -> Result<ArticleSource> {
            let sources: Vec<ArticleSource> = from_bytes(sources_bytes)?;
            sources
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("Article has no sources"))
        })
        .collect::<Result<_>>()?;

    let embeddings = generate_article_embeddings(&first_sources)
        .await
        .inspect_err(|e| error!("Re-classification embedding generation failed: {e}"))?;

    let mut tx = DB.begin().await?;
    for ((id, _, old_type, old_cat), source, embedding) in targets
        .iter()
        .zip(&first_sources)
        .zip(embeddings)
        .map(|((a, b), c)| (a, b, c))
    {
        let (article_type, category) = crate::server::embeddings::classify(&embedding).await?;

        let new_type = article_type.to_string();
        let new_cat = category.to_string();
        if new_type != *old_type || new_cat != *old_cat {
            info!(
                "[RECLASSIFY] '{}' {}/{} → {}/{}",
                source.title, old_type, old_cat, new_type, new_cat
            );
        }

        sqlx::query(
            "UPDATE articles SET embedding = ?, embedding_model = ?, article_type = ?, category = ?, updated_at = ? WHERE id = ?",
        )
        .bind(to_allocvec(&embedding)?)
        .bind(MODEL_NAME)
        .bind(new_type)
        .bind(new_cat)
        .bind(Utc::now().timestamp())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Embeddings generated with an older model will be regenerated.
pub async fn regenerate_stale_embeddings() -> Result<()> {
    let stale: Vec<(Uuid, Vec<u8>)> =
        sqlx::query_as("SELECT id, sources FROM articles WHERE embedding_model != ?")
            .bind(MODEL_NAME)
            .fetch_all(&*DB)
            .await?;

    if stale.is_empty() {
        return Ok(());
    }

    info!("Re-embedding {} articles for new model...", stale.len());

    let first_sources: Vec<ArticleSource> = stale
        .iter()
        .map(|(_, sources_bytes)| -> Result<ArticleSource> {
            let sources: Vec<ArticleSource> = from_bytes(sources_bytes)?;
            sources
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("Article has no sources"))
        })
        .collect::<Result<_>>()?;

    let embeddings = generate_article_embeddings(&first_sources)
        .await
        .inspect_err(|e| error!("Stale embedding regeneration failed: {e}"))?;

    let mut tx = DB.begin().await?;
    for ((id, _), embedding) in stale.iter().zip(embeddings) {
        sqlx::query(
            "UPDATE articles SET embedding = ?, embedding_model = ?, updated_at = ? WHERE id = ?",
        )
        .bind(to_allocvec(&embedding)?)
        .bind(MODEL_NAME)
        .bind(Utc::now().timestamp())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Finds articles that have haven't had a merged entry generated yet.
pub async fn get_regeneration_targets() -> Result<Vec<(Uuid, Vec<ArticleSource>)>> {
    sqlx::query_as::<_, (Uuid, Vec<u8>)>("SELECT id, sources FROM articles WHERE entry IS NULL")
        .fetch_all(&*DB)
        .await?
        .into_iter()
        .map(|(id, sources)| Ok((id, from_bytes(&sources)?)))
        .collect()
}

/// Saves the entry for an article.
pub async fn save_article_entry(article_id: Uuid, entry: &ArticleEntry) -> Result<()> {
    sqlx::query("UPDATE articles SET entry = ?, updated_at = ? WHERE id = ?")
        .bind(to_allocvec(entry)?)
        .bind(Utc::now().timestamp())
        .bind(article_id)
        .execute(&*DB)
        .await?;
    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub async fn set_article_status(article_id: Uuid, status: ArticleStatus) -> Result<()> {
    let binned_at = (status == ArticleStatus::Binned).then(|| Utc::now().timestamp());
    sqlx::query(
        "INSERT INTO user_articles (article_id, status, binned_at) VALUES (?, ?, ?)
         ON CONFLICT(article_id) DO UPDATE SET status = excluded.status, binned_at = excluded.binned_at",
    )
    .bind(article_id)
    .bind(status.to_string())
    .bind(binned_at)
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Sets a user rating (Like/Dislike) for an article.
pub async fn set_rating(article_id: Uuid, rating: Rating) -> Result<()> {
    sqlx::query(
        "INSERT INTO article_ratings (article_id, rating) VALUES (?, ?)
         ON CONFLICT(article_id) DO UPDATE SET rating = excluded.rating",
    )
    .bind(article_id)
    .bind(rating.to_string())
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Retrieves all articles with their user-specific status and ratings.
pub async fn get_user_articles() -> Result<Vec<ArticleData>> {
    sqlx::query(
        "SELECT ua.article_id, ua.status, ar.rating, a.sources, a.estimated_liked, a.entry,
                a.published, a.updated_at, a.article_type, a.category
         FROM user_articles ua
         JOIN articles a ON ua.article_id = a.id
         LEFT JOIN article_ratings ar ON ua.article_id = ar.article_id
         ORDER BY a.published DESC",
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| -> Result<ArticleData> {
        let id: Uuid = row.try_get("article_id")?;
        let status = row
            .try_get::<String, _>("status")?
            .parse()
            .map_err(|e| anyhow!("{e}"))?;
        let rating = row
            .try_get::<Option<String>, _>("rating")?
            .map(|s| s.parse())
            .transpose()
            .map_err(|e| anyhow!("{e}"))?;
        let art = StoredArticle {
            id,
            sources: from_bytes(&row.try_get::<Vec<u8>, _>("sources")?)?,
            estimated_liked: row.try_get::<f64, _>("estimated_liked")? as f32,
            entry: row
                .try_get::<Option<Vec<u8>>, _>("entry")?
                .map(|b| from_bytes(&b))
                .transpose()
                .unwrap_or_default(),
            published: DateTime::from_timestamp(row.try_get("published")?, 0).unwrap_or_default(),
            updated_at: DateTime::from_timestamp(row.try_get("updated_at")?, 0).unwrap_or_default(),
            article_type: row
                .try_get::<String, _>("article_type")?
                .parse()
                .unwrap_or(ArticleType::News),
            category: row
                .try_get::<String, _>("category")?
                .parse()
                .unwrap_or(Category::Technology),
        };
        Ok((id, status, rating, art))
    })
    .collect()
}

/// Deletes articles from `user_articles` that have been binned for longer than the specified days.
pub async fn cleanup_binned(days: i64) -> Result<()> {
    let cutoff = Utc::now().timestamp() - days * 86_400;
    sqlx::query("DELETE FROM user_articles WHERE status = 'Binned' AND binned_at < ?")
        .bind(cutoff)
        .execute(&*DB)
        .await?;
    Ok(())
}

/// Sets a rating for a generic item (source, feed, or tag).
pub async fn set_item_rating(key: &str, rating: Rating) -> Result<()> {
    sqlx::query(
        "INSERT INTO item_ratings (key, rating) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET rating = excluded.rating",
    )
    .bind(key)
    .bind(rating.to_string())
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Retrieves all generic item ratings.
pub async fn get_all_item_ratings() -> Result<HashMap<String, Rating>> {
    sqlx::query_as::<_, (String, String)>("SELECT key, rating FROM item_ratings")
        .fetch_all(&*DB)
        .await?
        .into_iter()
        .map(|(key, rating)| rating.parse().map(|r| (key, r)).map_err(|e| anyhow!("{e}")))
        .collect()
}
