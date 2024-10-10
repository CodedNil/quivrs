use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Serialize, Deserialize)]
struct FeedsConfig {
    rss: Vec<Rss>,
}

#[derive(Serialize, Deserialize)]
struct Rss {
    category: String,
    title: String,
    url: String,
}

pub async fn pull_articles() -> Result<()> {
    let config: FeedsConfig =
        ron::from_str(&fs::read_to_string("feeds.ron").await?).expect("Failed to parse feeds.ron");

    Ok(())
}
