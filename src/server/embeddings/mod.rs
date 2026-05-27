mod category;
mod importance;
mod region;
mod sentiment;

use crate::{
    server::database,
    shared::{Category, PendingSource, Region},
};
use anyhow::{Context, Result, anyhow, bail};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fmt::Write,
    sync::{LazyLock, Mutex},
};
use strum::IntoEnumIterator;
use surrealdb::types::RecordId;
use tracing::info;

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const EMBEDDING_MODEL_NAME: &str = "EmbeddingGemma300M";
const CLASSIFICATION_PROMPT: &str = "task: classification | query: ";

pub fn label_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(b"\0");
    hasher.update(EMBEDDING_MODEL_NAME.as_bytes());
    hasher.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

struct LabelDefinition {
    key: String,
    label_group: &'static str,
    label_value: String,
    hash: String,
    text: &'static str,
}

fn label_definitions() -> Vec<LabelDefinition> {
    let mut definitions = Vec::new();

    for category in Category::iter() {
        for (idx, text) in category::labels(category).iter().enumerate() {
            definitions.push(LabelDefinition {
                key: format!("category:{category}:{idx}"),
                label_group: "category",
                label_value: category.to_string(),
                hash: label_hash(&classification_input(text)),
                text,
            });
        }
    }

    for region in Region::iter() {
        for (idx, text) in region::labels(region).iter().enumerate() {
            definitions.push(LabelDefinition {
                key: format!("region:{region}:{idx}"),
                label_group: "region",
                label_value: region.to_string(),
                hash: label_hash(&classification_input(text)),
                text,
            });
        }
    }

    for (group, key, texts) in [
        ("sentiment", "positive", sentiment::labels("positive")),
        ("sentiment", "negative", sentiment::labels("negative")),
        ("importance", "important", importance::labels("important")),
        (
            "importance",
            "unimportant",
            importance::labels("unimportant"),
        ),
    ] {
        for (idx, text) in texts.iter().enumerate() {
            definitions.push(LabelDefinition {
                key: format!("{group}:{key}:{idx}"),
                label_group: group,
                label_value: key.to_string(),
                hash: label_hash(&classification_input(text)),
                text,
            });
        }
    }

    definitions
}

pub fn classification_input(text: &str) -> String {
    format!("{CLASSIFICATION_PROMPT}{text}")
}

static EMBEDDING_MODEL: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

pub fn article_text(s: &PendingSource, title_repeat: usize) -> String {
    std::iter::repeat_n(s.title.replace('\n', " "), title_repeat)
        .chain([
            s.tags.iter().take(8).join(", ").replace('\n', " "),
            s.summary
                .chars()
                .take(200)
                .collect::<String>()
                .replace('\n', " "),
        ])
        .filter(|field| !field.is_empty())
        .join(". ")
}

pub async fn generate_article_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let texts = texts.to_vec();
    let mut embeddings = tokio::task::spawn_blocking(move || {
        EMBEDDING_MODEL
            .lock()
            .map_err(|_| anyhow!("Embedding model lock poisoned"))?
            .embed(texts, None)
    })
    .await
    .context("Embedding worker task failed")??;

    for embedding in &mut embeddings {
        normalize(embedding);
    }
    Ok(embeddings)
}

fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v {
            *x /= norm;
        }
    }
}

async fn maintenance_label_embeddings() -> Result<()> {
    let definitions = label_definitions();
    let keys: HashSet<_> = definitions.iter().map(|def| def.key.clone()).collect();
    let cached_hashes = database::get_label_hashes().await?;
    let stale: Vec<_> = definitions
        .iter()
        .filter(|def| cached_hashes.get(&def.key) != Some(&def.hash))
        .collect();

    if !stale.is_empty() {
        info!("Refreshing {} label embeddings", stale.len());
        let texts: Vec<_> = stale
            .iter()
            .map(|def| classification_input(def.text))
            .collect();
        let embeddings = generate_article_embeddings(&texts).await?;
        let records: Vec<_> = stale
            .into_iter()
            .zip(embeddings)
            .map(|(def, embedding)| database::LabelEmbeddingRecord {
                id: RecordId::new("label_embeddings", def.key.as_str()),
                label_group: def.label_group.to_string(),
                label_value: def.label_value.clone(),
                hash: def.hash.clone(),
                text: def.text.to_string(),
                embedding,
            })
            .collect();

        database::upsert_label_embeddings(&records).await?;
    }

    database::delete_label_embeddings_except(&keys).await?;
    Ok(())
}

pub async fn maintenance_embeddings() -> Result<()> {
    maintenance_label_embeddings().await?;
    maintenance_article_embeddings().await?;
    Ok(())
}

async fn maintenance_article_embeddings() -> Result<()> {
    let current_model = EMBEDDING_MODEL_NAME;

    for table in ["pending_sources", "user_articles"] {
        let stale = database::get_stale_embedding_records(table, current_model).await?;
        if stale.is_empty() {
            continue;
        }

        info!("Updating embeddings for {} records in {table}", stale.len());
        for chunk in stale.chunks(100) {
            let texts: Vec<String> = chunk.iter().map(|r| r.embedding_text.clone()).collect();
            let new_embeddings = generate_article_embeddings(&texts).await?;
            let updates: Vec<_> = chunk
                .iter()
                .zip(new_embeddings)
                .map(|(record, embedding)| database::EmbeddingUpdate {
                    id: record.id.clone(),
                    embedding,
                })
                .collect();
            database::update_record_embeddings(&updates, current_model).await?;
        }
    }

    Ok(())
}

pub async fn classify(article_embedding: &[f32]) -> Result<(Category, Region, f32, f32)> {
    let scores = database::get_label_scores(article_embedding).await?;

    Ok((
        best_label(&scores, "category")?,
        best_label(&scores, "region")?,
        binary_label_score(&scores, "sentiment", "positive")?,
        binary_label_score(&scores, "importance", "important")?,
    ))
}

fn best_label<T>(scores: &[database::LabelScore], group: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    scores
        .iter()
        .filter(|row| row.label_group == group)
        .max_by(|a, b| a.similarity.total_cmp(&b.similarity))
        .ok_or_else(|| anyhow!("No {group} label embeddings found"))?
        .label_value
        .parse()
        .map_err(Into::into)
}

fn binary_label_score(
    rows: &[database::LabelScore],
    label_group: &str,
    positive_value: &str,
) -> Result<f32> {
    let mut positive = 0.0;
    let mut total = 0.0;
    for row in rows.iter().filter(|row| row.label_group == label_group) {
        let weight = (row.similarity * 10.0).exp();
        if row.label_value == positive_value {
            positive += weight;
        }
        total += weight;
    }

    if total == 0.0 {
        bail!("No {label_group} label embeddings found");
    }

    Ok(positive / total)
}
