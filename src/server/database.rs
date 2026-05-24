use crate::server::embeddings::{EMBEDDING_MODEL_NAME, generate_article_embeddings};
use crate::shared::{ArticleStatus, Category, PendingSource, Rating};
use crate::{
    server::embeddings::embed_label_texts,
    shared::{Article, ArticleSource},
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use postcard::{from_bytes, to_allocvec};
use sqlx::{
    QueryBuilder, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    types::Json,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    str::FromStr,
    sync::LazyLock,
};
use tracing::info;
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

/// Inserts a newly found source.
pub async fn insert_source(source: &PendingSource) -> Result<()> {
    let mut tx = DB.begin().await?;
    sqlx::query!(
        "INSERT INTO pending_sources
             (url, domain, title, summary, content, tags, images, published, embedding, embedding_model, embedding_text, category)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        source.url,
        source.domain,
        source.title,
        source.summary,
        source.content,
        Json(source.tags.clone()),
        Json(source.images.clone()),
        source.published,
        to_allocvec(&source.embedding)?,
        source.embedding_model,
        source.embedding_text,
        source.category,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("INSERT INTO article_urls (url) VALUES (?)", source.url)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub async fn set_article_status(id: Uuid, status: ArticleStatus) -> Result<()> {
    let binned_at = (status == ArticleStatus::Binned).then(Utc::now);
    sqlx::query!(
        "UPDATE user_articles SET status = ?, binned_at = ? WHERE id = ?",
        status,
        binned_at,
        id,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Sets a user rating for an article.
pub async fn set_rating(id: Uuid, rating: Rating) -> Result<()> {
    sqlx::query!(
        "UPDATE user_articles SET rating = ? WHERE id = ?",
        rating,
        id,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Retrieves all articles with their user-specific status and ratings.
pub async fn get_user_articles() -> Result<Vec<Article>> {
    sqlx::query!(
        r#"SELECT
            id              as "id!: Uuid",

            sources         as "sources!: Json<Vec<ArticleSource>>",
            title           as "title!: String",
            description     as "description!: String",
            content         as "content!: String",
            sidebar         as "sidebar!: String",
            thumbnail       as "thumbnail!: String",
            published       as "published!: DateTime<Utc>",
            category        as "category!: Category",

            status          as "status!: ArticleStatus",
            binned_at       as "binned_at: DateTime<Utc>",
            rating          as "rating: Rating",

            embedding       as "embedding!: Vec<u8>",
            embedding_text  as "embedding_text!: String",
            embedding_model as "embedding_model!: String"

         FROM user_articles
         ORDER BY published DESC"#,
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| {
        Ok(Article {
            id: row.id,

            sources: row.sources.0,
            title: row.title,
            description: row.description,
            content: row.content,
            sidebar: row.sidebar,
            thumbnail: row.thumbnail,
            published: row.published,
            category: row.category,

            status: row.status,
            binned_at: row.binned_at,
            rating: row.rating,

            embedding: from_bytes(&row.embedding)?,
            embedding_text: row.embedding_text,
            embedding_model: row.embedding_model,
        })
    })
    .collect()
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

pub async fn get_or_refresh_label_embeddings_batch(
    entries: &[(String, String, String)],
) -> Result<Vec<Vec<f32>>> {
    let mut cached: HashMap<String, (String, Vec<f32>)> = sqlx::query!(
        r#"SELECT key as "key!", hash as "hash!", embedding as "embedding!" FROM label_embeddings"#
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| Ok((row.key, (row.hash, from_bytes::<Vec<f32>>(&row.embedding)?))))
    .collect::<Result<_>>()?;

    let stale: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, (k, h, _))| cached.get(k).is_none_or(|(ch, _)| ch != h))
        .map(|(i, _)| i)
        .collect();

    if !stale.is_empty() {
        let texts: Vec<String> = stale.iter().map(|&i| entries[i].2.clone()).collect();
        for (&i, emb) in stale.iter().zip(embed_label_texts(&texts).await?) {
            let (key, hash, _) = &entries[i];
            let bytes = to_allocvec(&emb)?;
            sqlx::query!(
                "INSERT INTO label_embeddings (key, hash, embedding) VALUES (?, ?, ?)
                 ON CONFLICT(key) DO UPDATE SET hash = excluded.hash, embedding = excluded.embedding",
                key, hash, bytes
            ).execute(&*DB).await?;
            cached.insert(key.clone(), (hash.clone(), emb));
        }
    }

    Ok(entries
        .iter()
        .map(|(k, _, _)| cached.remove(k).unwrap().1)
        .collect())
}

/// Retrieves all articles with their embeddings for preference calculation.
pub async fn get_rated_article_embeddings() -> Result<Vec<(Rating, Vec<f32>)>> {
    let rows = sqlx::query!(
        r#"SELECT rating as "rating!: Rating", embedding
         FROM user_articles
         WHERE rating IS NOT NULL"#
    )
    .fetch_all(&*DB)
    .await?;

    let mut results = Vec::new();
    for row in rows {
        results.push((row.rating, from_bytes(&row.embedding)?));
    }
    Ok(results)
}

/// Retrieves all pending sources.
pub async fn get_pending_sources() -> Result<Vec<PendingSource>> {
    sqlx::query!(
        r#"SELECT
            url as "url!", domain as "domain!", title as "title!", summary as "summary!", content as "content!",
            tags as "tags!: Json<Vec<String>>",
            images as "images!: Json<Vec<(String, String)>>",
            published as "published!: DateTime<Utc>",
            embedding, embedding_model, embedding_text,
            category as "category!: Category"
         FROM pending_sources"#
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| {
        Ok(PendingSource {
            url: row.url,
            domain: row.domain,
            title: row.title,
            summary: row.summary,
            content: row.content,
            tags: row.tags.0,
            images: row.images.0,
            published: row.published,
            embedding: from_bytes(&row.embedding)?,
            embedding_model: row.embedding_model,
            embedding_text: row.embedding_text,
            category: row.category,
        })
    })
    .collect()
}

/// Retrieves the count of articles per category that have a 'New' status.
pub async fn get_category_article_counts() -> Result<HashMap<Category, i64>> {
    let rows = sqlx::query!(
        r#"SELECT category as "category!: Category", COUNT(*) as count
         FROM user_articles
         WHERE status = 'New'
         GROUP BY category"#
    )
    .fetch_all(&*DB)
    .await?;
    Ok(rows.into_iter().map(|r| (r.category, r.count)).collect())
}

/// Inserts a promoted article and removes its sources from pending.
pub async fn insert_promoted_article(article: &Article, source_urls: &[String]) -> Result<()> {
    let mut tx = DB.begin().await?;

    sqlx::query!(
        "INSERT INTO user_articles
             (id, sources, title, description, content, sidebar, thumbnail, published, category, status, embedding, embedding_text, embedding_model)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        article.id,
        Json(article.sources.clone()),
        article.title,
        article.description,
        article.content,
        article.sidebar,
        article.thumbnail,
        article.published,
        article.category,
        article.status,
        to_allocvec(&article.embedding)?,
        article.embedding_text,
        article.embedding_model,
    )
    .execute(&mut *tx)
    .await?;

    let mut builder = QueryBuilder::new("DELETE FROM pending_sources WHERE url IN (");
    let mut separated = builder.separated(", ");
    for url in source_urls {
        separated.push_bind(url);
    }
    separated.push_unseparated(")");
    builder.build().execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}

/// Maintenance task to re-embed articles if the model has changed and purge old sources.
pub async fn maintenance_embeddings() -> Result<()> {
    let current_model = EMBEDDING_MODEL_NAME;

    // Update pending_sources
    let stale_pending = sqlx::query!(
        "SELECT url, embedding_text FROM pending_sources WHERE embedding_model != ?",
        current_model
    )
    .fetch_all(&*DB)
    .await?;

    if !stale_pending.is_empty() {
        info!(
            "Updating embeddings for {} pending sources",
            stale_pending.len()
        );
        for chunk in stale_pending.chunks(100) {
            let texts: Vec<String> = chunk.iter().map(|r| r.embedding_text.clone()).collect();
            let new_embs = generate_article_embeddings(&texts).await?;
            for (row, emb) in chunk.iter().zip(new_embs.iter()) {
                sqlx::query!(
                    "UPDATE pending_sources SET embedding = ?, embedding_model = ? WHERE url = ?",
                    to_allocvec(emb)?,
                    current_model,
                    row.url
                )
                .execute(&*DB)
                .await?;
            }
        }
    }

    // Update user_articles
    let stale_articles = sqlx::query!(
        "SELECT id, embedding_text FROM user_articles WHERE embedding_model != ?",
        current_model
    )
    .fetch_all(&*DB)
    .await?;

    if !stale_articles.is_empty() {
        info!(
            "Updating embeddings for {} user articles",
            stale_articles.len()
        );
        for chunk in stale_articles.chunks(100) {
            let texts: Vec<String> = chunk.iter().map(|r| r.embedding_text.clone()).collect();
            let new_embs = generate_article_embeddings(&texts).await?;
            for (row, emb) in chunk.iter().zip(new_embs.iter()) {
                sqlx::query!(
                    "UPDATE user_articles SET embedding = ?, embedding_model = ? WHERE id = ?",
                    to_allocvec(emb)?,
                    current_model,
                    row.id
                )
                .execute(&*DB)
                .await?;
            }
        }
    }

    // Purge articles older than 2 weeks
    let two_weeks_ago = Utc::now() - chrono::Duration::weeks(2);
    let deleted = sqlx::query!(
        "DELETE FROM pending_sources WHERE published < ?",
        two_weeks_ago
    )
    .execute(&*DB)
    .await?;

    if deleted.rows_affected() > 0 {
        info!("Purged {} stale pending sources", deleted.rows_affected());
    }

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
