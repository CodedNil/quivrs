use crate::server::{
    embeddings::{
        MODEL_NAME, article_text, category_label, classify, embed_label_texts,
        generate_article_embeddings, label_hash, seed_label_cache,
    },
    parsers::fetch_page_content,
};
use crate::shared::{
    ArticleData, ArticleEntry, ArticleSource, ArticleStatus, Category, Rating, StoredArticle,
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
use strum::IntoEnumIterator;
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
    init_label_embeddings().await?;
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
    pub category: Category,
    pub embedding: Vec<f32>,
    pub published: i64,
}

#[derive(sqlx::FromRow)]
struct ReclassifyRow {
    id: Uuid,
    sources: Json<Vec<ArticleSource>>,
    category: Category,
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
        r#"SELECT id as "id!: Uuid", title, category as "category!: Category", embedding, published
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
            category: row.category,
            embedding: from_bytes(&row.embedding)?,
            published: row.published,
        })
    })
    .collect()
}

/// Inserts a new article and its initial metadata.
pub async fn insert_article(
    source: &ArticleSource,
    embedding: &[f32],
    category: Category,
) -> Result<()> {
    let id = Uuid::new_v4();
    let embedding_bytes = to_allocvec(embedding)?;

    let published = source.published;
    let mut tx = DB.begin().await?;
    sqlx::query!(
        "INSERT INTO articles
             (id, title, sources, estimated_liked, entry, embedding, embedding_model, published, updated_at, category)
         VALUES (?, ?, ?, 0.0, NULL, ?, ?, ?, strftime('%s', 'now'), ?)",
        id,
        source.title,
        Json(vec![source]),
        embedding_bytes,
        MODEL_NAME,
        published,
        category,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("INSERT INTO article_urls (url) VALUES (?)", source.url)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "INSERT INTO user_articles (article_id, status) VALUES (?, ?)",
        id,
        ArticleStatus::New,
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
    sqlx::query!(
        "UPDATE articles SET sources = ?, entry = NULL WHERE id = ?",
        sources_bytes,
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

    let mut builder = QueryBuilder::new("SELECT id, sources, category FROM articles WHERE id IN (");
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
        .map(|row| {
            row.sources
                .0
                .first()
                .cloned()
                .ok_or_else(|| anyhow!("Article has no sources"))
        })
        .collect::<Result<_>>()?;

    let embeddings = generate_article_embeddings(&first_sources)
        .await
        .inspect_err(|e| error!("Re-classification embedding generation failed: {e}"))?;

    let mut tx = DB.begin().await?;
    for ((row, source), embeddings) in targets.iter().zip(&first_sources).zip(embeddings) {
        let category = classify(&embeddings).await?;
        if category != row.category {
            info!(
                "[RECLASSIFY] '{}' {} → {}",
                article_text(source, 1),
                row.category,
                category
            );
        }
        let embedding_bytes = to_allocvec(&embeddings)?;
        sqlx::query!(
            "UPDATE articles SET embedding = ?, embedding_model = ?, category = ? WHERE id = ?",
            embedding_bytes,
            MODEL_NAME,
            category,
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
         WHERE embedding_model != ? OR embedding IS NULL"#,
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

    let embeddings = generate_article_embeddings(&first_sources)
        .await
        .inspect_err(|e| error!("Stale embedding regeneration failed: {e}"))?;

    let mut tx = DB.begin().await?;
    for (row, embedding) in stale.iter().zip(embeddings) {
        let embedding_bytes = to_allocvec(&embedding)?;
        sqlx::query!(
            "UPDATE articles SET embedding = ?, embedding_model = ? WHERE id = ?",
            embedding_bytes,
            MODEL_NAME,
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
    sqlx::query!(
        "UPDATE articles SET entry = ? WHERE id = ?",
        entry_bytes,
        article_id,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Updates the user status (new, read, binned) for an article.
pub async fn set_article_status(article_id: Uuid, status: ArticleStatus) -> Result<()> {
    let binned_at = (status == ArticleStatus::Binned).then(Utc::now);
    sqlx::query!(
        "INSERT INTO user_articles (article_id, status, binned_at) VALUES (?, ?, ?)
         ON CONFLICT(article_id) DO UPDATE SET status = excluded.status, binned_at = excluded.binned_at",
        article_id,
        status,
        binned_at,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Sets a user rating for an article.
pub async fn set_rating(article_id: Uuid, rating: Rating) -> Result<()> {
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
            ua.status      as "status!: ArticleStatus",
            ar.rating      as "rating: Rating",
            a.sources      as "sources!: Json<Vec<ArticleSource>>",
            a.estimated_liked,
            a.entry        as "entry: Json<ArticleEntry>",
            a.published    as "published!: DateTime<Utc>",
            a.updated_at   as "updated_at!: DateTime<Utc>",
            a.category     as "category!: Category"
         FROM user_articles ua
         JOIN articles a ON ua.article_id = a.id
         LEFT JOIN article_ratings ar ON ua.article_id = ar.article_id
         ORDER BY a.published DESC"#,
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| {
        let article = StoredArticle {
            id: row.id,
            sources: row.sources.0,
            estimated_liked: row.estimated_liked as f32,
            entry: row.entry.map(|e| e.0),
            published: row.published,
            updated_at: row.updated_at,
            category: row.category,
        };
        Ok(ArticleData {
            id: row.id,
            status: row.status,
            rating: row.rating,
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

async fn upsert_label_embedding(key: &str, hash: &str, embedding: &[f32]) -> Result<()> {
    let bytes = to_allocvec(embedding)?;
    sqlx::query!(
        "INSERT INTO label_embeddings (key, hash, embedding) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET hash = excluded.hash, embedding = excluded.embedding",
        key,
        hash,
        bytes,
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

/// Loads label embeddings from DB, regenerating only stale or missing entries.
async fn init_label_embeddings() -> Result<()> {
    let entries: Vec<(String, String, String)> = Category::iter()
        .map(|c| {
            let text = category_label(c);
            (
                format!("category_{c}"),
                label_hash(text),
                format!("title: none | text: {text}"),
            )
        })
        .collect();

    let mut cached: HashMap<String, (String, Vec<f32>)> = sqlx::query!(
        r#"SELECT key as "key!", hash as "hash!", embedding as "embedding!" FROM label_embeddings"#,
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|row| Ok((row.key, (row.hash, from_bytes::<Vec<f32>>(&row.embedding)?))))
    .collect::<Result<_>>()?;

    let stale: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, (key, hash, _))| cached.get(key).is_none_or(|(h, _)| h != hash))
        .map(|(i, _)| i)
        .collect();

    if !stale.is_empty() {
        info!("Regenerating {} stale label embeddings...", stale.len());
        let texts: Vec<String> = stale.iter().map(|&i| entries[i].2.clone()).collect();
        for (&i, emb) in stale.iter().zip(embed_label_texts(&texts).await?) {
            let (key, hash, _) = &entries[i];
            upsert_label_embedding(key, hash, &emb).await?;
            cached.insert(key.clone(), (hash.clone(), emb));
        }
    }

    seed_label_cache(
        entries
            .iter()
            .map(|(key, _, _)| cached.remove(key).unwrap().1)
            .collect(),
    )
    .await;
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

/// Resets an article's generated entry to NULL and refreshes its sources.
pub async fn regenerate_article(article_id: Uuid) -> Result<()> {
    let raw = sqlx::query!("SELECT sources FROM articles WHERE id = ?", article_id)
        .fetch_one(&*DB)
        .await?
        .sources;

    let sources: Vec<ArticleSource> = from_bytes(&raw)?;
    let mut updated_sources = Vec::with_capacity(sources.len());

    for source in sources {
        let updated = fetch_page_content(&source.url).await;

        match updated {
            Ok(Some(s)) => updated_sources.push(s),
            _ => updated_sources.push(source),
        }
    }

    let sources_bytes = to_allocvec(&updated_sources)?;

    sqlx::query!(
        "UPDATE articles SET entry = NULL, sources = ? WHERE id = ?",
        sources_bytes,
        article_id,
    )
    .execute(&*DB)
    .await?;

    if let Err(err) = super::articles::regenerate_articles().await {
        error!("Article regeneration failed: {err}");
    }

    Ok(())
}
