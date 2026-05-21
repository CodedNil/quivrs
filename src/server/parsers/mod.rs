pub mod feeds;
pub mod social;
pub mod websites;

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::PathBuf;
use tokio::fs;

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

pub fn get_cache_path(url: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join("quivrs").join(format!(
        "{}.{}",
        sha256_hex(url.as_bytes()),
        extension
    ))
}

pub async fn get_cached_or_fetch_ext(url: &str, ext: &str) -> Result<String> {
    let cache_path = get_cache_path(url, ext);

    if let Ok(bytes) = fs::read(&cache_path).await {
        return Ok(String::from_utf8_lossy(&bytes).into_owned());
    }

    let text = crate::server::HTTP_CLIENT
        .get(url)
        .send()
        .await?
        .text()
        .await?;
    if let Some(dir) = cache_path.parent()
        && fs::create_dir_all(dir).await.is_ok()
    {
        let _ = fs::write(&cache_path, text.as_bytes()).await;
    }
    Ok(text)
}
