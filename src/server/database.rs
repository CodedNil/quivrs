use crate::server::embeddings::{MODEL_NAME, classify, generate_article_embeddings};
use crate::shared::{
    ArticleData, ArticleEntry, ArticleSource, ArticleStatus, ArticleType, Category, Rating,
    StoredArticle,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use postcard::{from_bytes, to_allocvec};
use sqlx::{
    QueryBuilder, SqlitePool,
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
    sqlx::migrate!().run(&*DB).await?;
    Ok(())
}

fn first_source(blob: &[u8]) -> Result<ArticleSource> {
    from_bytes::<Vec<ArticleSource>>(blob)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Article has no sources"))
}

pub struct EmbeddingCandidate {
    pub id: Uuid,
    pub title: String,
    pub embedding_similarity: Vec<f32>,
}

// Only needed for the dynamic QueryBuilder IN-clause in reclassify_articles.
#[derive(sqlx::FromRow)]
struct ReclassifyRow {
    id: Uuid,
    sources: Vec<u8>,
    article_type: String,
    category: String,
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
        .build_query_scalar()
        .fetch_all(&*DB)
        .await?
        .into_iter()
        .collect();
    Ok(urls.difference(&existing).cloned().collect())
}

/// Records URLs as permanently skipped so they are never re-fetched.
pub async fn mark_urls_dismissed(urls: &[String]) -> Result<()> {
    if urls.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::new("INSERT OR IGNORE INTO article_urls (url) ");
    builder.push_values(urls.iter(), |mut b, url| {
        b.push_bind(url);
    });
    builder.build().execute(&*DB).await?;
    Ok(())
}

/// Retrieves articles within a time window for similarity comparison.
pub async fn get_embedding_candidates(
    window_start: i64,
    window_end: i64,
) -> Result<Vec<EmbeddingCandidate>> {
    sqlx::query!(
        r#"SELECT id as "id!: Uuid", title, embedding_similarity
         FROM articles WHERE published BETWEEN ? AND ? AND embedding_model = ?"#,
        window_start,
        window_end,
        MODEL_NAME,
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| {
        Ok(EmbeddingCandidate {
            id: row.id,
            title: row.title,
            embedding_similarity: from_bytes(&row.embedding_similarity)?,
        })
    })
    .collect()
}

/// Inserts a new article and its initial metadata.
pub async fn insert_article(
    source: &ArticleSource,
    embedding_similarity: &[f32],
    embedding_classification: &[f32],
    article_type: ArticleType,
    category: Category,
) -> Result<()> {
    let id = Uuid::new_v4();
    let sources_bytes = to_allocvec(std::slice::from_ref(source))?;
    let similarity_bytes = to_allocvec(embedding_similarity)?;
    let classification_bytes = to_allocvec(embedding_classification)?;

    let published = source.published.timestamp();
    let updated_at = Utc::now().timestamp();
    let article_type = article_type.to_string();
    let category = category.to_string();
    let mut tx = DB.begin().await?;
    sqlx::query!(
        "INSERT INTO articles
             (id, title, sources, estimated_liked, entry, embedding_similarity,
              embedding_classification, embedding_model, published, updated_at,
              article_type, category)
         VALUES (?, ?, ?, 0.0, NULL, ?, ?, ?, ?, ?, ?, ?)",
        id,
        source.title,
        sources_bytes,
        similarity_bytes,
        classification_bytes,
        MODEL_NAME,
        published,
        updated_at,
        article_type,
        category,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("INSERT INTO article_urls (url) VALUES (?)", source.url)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "INSERT INTO user_articles (article_id, status) VALUES (?, 'New')",
        id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Merges a new source into an existing article.
pub async fn merge_into_article(article_id: Uuid, source: &ArticleSource) -> Result<()> {
    let mut tx = DB.begin().await?;

    let raw = sqlx::query!("SELECT sources FROM articles WHERE id = ?", article_id)
        .fetch_one(&mut *tx)
        .await?
        .sources;
    let mut sources: Vec<ArticleSource> = from_bytes(&raw)?;
    sources.push(source.clone());

    let sources_bytes = to_allocvec(&sources)?;
    let updated_at = Utc::now().timestamp();
    sqlx::query!(
        "UPDATE articles SET sources = ?, entry = NULL, updated_at = ? WHERE id = ?",
        sources_bytes,
        updated_at,
        article_id,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT OR IGNORE INTO article_urls (url) VALUES (?)",
        source.url,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Reruns classification on a set of articles.
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

    let targets: Vec<ReclassifyRow> = builder.build_query_as().fetch_all(&*DB).await?;
    if targets.is_empty() {
        return Ok(());
    }

    info!("Re-classifying {} articles...", targets.len());

    let first_sources: Vec<ArticleSource> = targets
        .iter()
        .map(|row| first_source(&row.sources))
        .collect::<Result<_>>()?;

    let (similarity_embs, classification_embs) = generate_article_embeddings(&first_sources)
        .await
        .inspect_err(|e| error!("Re-classification embedding generation failed: {e}"))?;

    let mut tx = DB.begin().await?;
    for (((row, source), sim_emb), cls_emb) in targets
        .iter()
        .zip(&first_sources)
        .zip(similarity_embs)
        .zip(classification_embs)
    {
        let (article_type, category) = classify(&cls_emb).await?;
        let new_type = article_type.to_string();
        let new_cat = category.to_string();
        if new_type != row.article_type || new_cat != row.category {
            info!(
                "[RECLASSIFY] '{}' {}/{} → {}/{}",
                source.title, row.article_type, row.category, new_type, new_cat
            );
        }
        let similarity_bytes = to_allocvec(&sim_emb)?;
        let classification_bytes = to_allocvec(&cls_emb)?;
        let updated_at = Utc::now().timestamp();
        sqlx::query!(
            "UPDATE articles SET embedding_similarity = ?, embedding_classification = ?,
             embedding_model = ?, article_type = ?, category = ?, updated_at = ? WHERE id = ?",
            similarity_bytes,
            classification_bytes,
            MODEL_NAME,
            new_type,
            new_cat,
            updated_at,
            row.id,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Embeddings generated with an older model will be regenerated.
pub async fn regenerate_stale_embeddings() -> Result<()> {
    let stale = sqlx::query!(
        r#"SELECT id as "id!: Uuid", sources FROM articles
         WHERE embedding_model != ? OR embedding_classification IS NULL"#,
        MODEL_NAME,
    )
    .fetch_all(&*DB)
    .await?;

    if stale.is_empty() {
        return Ok(());
    }

    info!("Re-embedding {} articles for new model...", stale.len());

    let first_sources: Vec<ArticleSource> = stale
        .iter()
        .map(|row| first_source(&row.sources))
        .collect::<Result<_>>()?;

    let (similarity_embs, classification_embs) = generate_article_embeddings(&first_sources)
        .await
        .inspect_err(|e| error!("Stale embedding regeneration failed: {e}"))?;

    let mut tx = DB.begin().await?;
    for ((row, sim_emb), cls_emb) in stale.iter().zip(similarity_embs).zip(classification_embs) {
        let similarity_bytes = to_allocvec(&sim_emb)?;
        let classification_bytes = to_allocvec(&cls_emb)?;
        let updated_at = Utc::now().timestamp();
        sqlx::query!(
            "UPDATE articles SET embedding_similarity = ?, embedding_classification = ?,
             embedding_model = ?, updated_at = ? WHERE id = ?",
            similarity_bytes,
            classification_bytes,
            MODEL_NAME,
            updated_at,
            row.id,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Finds articles that haven't had a merged entry generated yet.
pub async fn get_regeneration_targets() -> Result<Vec<(Uuid, Vec<ArticleSource>)>> {
    sqlx::query!(r#"SELECT id as "id!: Uuid", sources FROM articles WHERE entry IS NULL"#,)
        .fetch_all(&*DB)
        .await?
        .into_iter()
        .map(|row| Ok((row.id, from_bytes(&row.sources)?)))
        .collect()
}

/// Saves the entry for an article.
pub async fn save_article_entry(article_id: Uuid, entry: &ArticleEntry) -> Result<()> {
    let entry_bytes = to_allocvec(entry)?;
    let updated_at = Utc::now().timestamp();
    sqlx::query!(
        "UPDATE articles SET entry = ?, updated_at = ? WHERE id = ?",
        entry_bytes,
        updated_at,
        article_id,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub async fn set_article_status(article_id: Uuid, status: ArticleStatus) -> Result<()> {
    let status_str = status.to_string();
    let binned_at = (status == ArticleStatus::Binned).then(|| Utc::now().timestamp());
    sqlx::query!(
        "INSERT INTO user_articles (article_id, status, binned_at) VALUES (?, ?, ?)
         ON CONFLICT(article_id) DO UPDATE SET status = excluded.status, binned_at = excluded.binned_at",
        article_id,
        status_str,
        binned_at,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Sets a user rating for an article.
pub async fn set_rating(article_id: Uuid, rating: Rating) -> Result<()> {
    let rating = rating.to_string();
    sqlx::query!(
        "INSERT INTO article_ratings (article_id, rating) VALUES (?, ?)
         ON CONFLICT(article_id) DO UPDATE SET rating = excluded.rating",
        article_id,
        rating,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Retrieves all articles with their user-specific status and ratings.
pub async fn get_user_articles() -> Result<Vec<ArticleData>> {
    sqlx::query!(
        r#"SELECT
            ua.article_id  as "id!: Uuid",
            ua.status      as "status!",
            ar.rating,
            a.sources      as "sources!",
            a.estimated_liked,
            a.entry,
            a.published    as "published!",
            a.updated_at   as "updated_at!",
            a.article_type as "article_type!",
            a.category     as "category!"
         FROM user_articles ua
         JOIN articles a ON ua.article_id = a.id
         LEFT JOIN article_ratings ar ON ua.article_id = ar.article_id
         ORDER BY a.published DESC"#,
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| -> Result<ArticleData> {
        let status = row.status.parse().map_err(|e| anyhow!("{e}"))?;
        let rating = row
            .rating
            .map(|s| s.parse())
            .transpose()
            .map_err(|e| anyhow!("{e}"))?;
        let article = StoredArticle {
            id: row.id,
            sources: from_bytes(&row.sources)?,
            estimated_liked: row.estimated_liked as f32,
            entry: row
                .entry
                .map(|b| from_bytes(&b))
                .transpose()
                .unwrap_or_default(),
            published: DateTime::from_timestamp(row.published, 0).unwrap_or_default(),
            updated_at: DateTime::from_timestamp(row.updated_at, 0).unwrap_or_default(),
            article_type: row.article_type.parse().unwrap_or(ArticleType::News),
            category: row.category.parse().unwrap_or(Category::Technology),
        };
        Ok(ArticleData {
            id: row.id,
            status,
            rating,
            article,
        })
    })
    .collect()
}

/// Deletes articles from `user_articles` that have been binned for longer than the specified days.
pub async fn cleanup_binned(days: i64) -> Result<()> {
    let cutoff = Utc::now().timestamp() - days * 86_400;
    sqlx::query!(
        "DELETE FROM user_articles WHERE status = 'Binned' AND binned_at < ?",
        cutoff,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Sets a rating for a generic item (source, feed, or tag).
pub async fn set_item_rating(key: &str, rating: Rating) -> Result<()> {
    let rating = rating.to_string();
    sqlx::query!(
        "INSERT INTO item_ratings (key, rating) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET rating = excluded.rating",
        key,
        rating,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Retrieves all generic item ratings.
pub async fn get_all_item_ratings() -> Result<HashMap<String, Rating>> {
    sqlx::query!(r#"SELECT key as "key!", rating FROM item_ratings"#)
        .fetch_all(&*DB)
        .await?
        .into_iter()
        .map(|row| {
            row.rating
                .parse()
                .map(|r| (row.key, r))
                .map_err(|e| anyhow!("{e}"))
        })
        .collect()
}
