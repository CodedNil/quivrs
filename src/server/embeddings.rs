use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::LazyLock;
use tokio::sync::Mutex;

static STATE: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::EmbeddingGemma300M).with_show_download_progress(true),
        )
        .expect("Failed to load embedding model"),
    )
});

pub async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    STATE.lock().await.embed(texts, None)
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
