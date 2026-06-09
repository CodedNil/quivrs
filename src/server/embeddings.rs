use crate::{server::database, shared::PendingSource};
use anyhow::{Context, Result, anyhow};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use rayon::prelude::*;
use std::sync::{LazyLock, Mutex};
use tracing::info;

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300MQ4;
pub const EMBEDDING_MODEL_NAME: &str = "google/embeddinggemma-300mq4";
pub const EMBEDDING_DIMENSIONS: usize = 512;

pub const EMBEDDING_TITLE_REPEAT: usize = 5;

pub fn embedding_model_id() -> String {
    format!("{EMBEDDING_MODEL_NAME}:{EMBEDDING_DIMENSIONS}")
}

static EMBEDDING_MODEL: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

pub fn article_text(source: &PendingSource, title_repeat: usize) -> String {
    std::iter::repeat_n(source.title.replace('\n', " "), title_repeat)
        .chain([
            source.tags.iter().take(8).join(", ").replace('\n', " "),
            source
                .summary
                .chars()
                .take(200)
                .collect::<String>()
                .replace('\n', " "),
        ])
        .filter(|field| !field.is_empty())
        .join(". ")
}

pub async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let texts_vec = texts.to_vec();
    let mut embeddings = tokio::task::spawn_blocking(move || {
        EMBEDDING_MODEL
            .lock()
            .map_err(|_| anyhow!("Embedding model lock poisoned"))?
            .embed(texts_vec, None)
    })
    .await
    .context("Embedding worker task failed")??;

    embeddings.par_iter_mut().for_each(|embedding| {
        embedding.truncate(EMBEDDING_DIMENSIONS);

        let norm: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if norm > f32::EPSILON {
            let inv_norm = 1.0 / norm;
            for x in embedding {
                *x *= inv_norm;
            }
        }
    });
    Ok(embeddings)
}

pub async fn maintenance_article_embeddings() -> Result<()> {
    for table in [
        database::EmbeddingTable::PendingSources,
        database::EmbeddingTable::Articles,
    ] {
        let stale = database::get_stale_embedding_records(table).await?;
        if stale.is_empty() {
            continue;
        }

        info!(
            "Updating embeddings for {} records in {}",
            stale.len(),
            table.name()
        );
        for chunk in stale.chunks(100) {
            let texts: Vec<String> = chunk.iter().map(|(_, text)| text.clone()).collect();
            let new_embeddings = generate_embeddings(&texts).await?;
            let updates: Vec<_> = chunk
                .iter()
                .zip(new_embeddings)
                .map(|((id, _), embedding)| (id.clone(), embedding))
                .collect();
            database::update_record_embeddings(table, &updates).await?;
        }
    }

    Ok(())
}
