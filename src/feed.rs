use anyhow::{Ok, Result};

struct FeedsConfig {
    rss: Vec<Rss>,
}

struct Rss {
    category: String,
    title: String,
    url: String,
}

fn default_feeds() -> Vec<Rss> {
    vec![
        Rss {
            category: "Informative".to_string(),
            title: "The Verge".to_string(),
            url: "https://www.theverge.com/rss/index.xml".to_string(),
        },
        Rss {
            category: "Informative".to_string(),
            title: "Veritasium".to_string(),
            url: "https://www.youtube.com/feeds/videos.xml?channel_id=UCHnyfMqiRRG1u-2MsSQLbXA"
                .to_string(),
        },
    ]
}

pub async fn pull_articles() -> Result<()> {
    let feeds = default_feeds();
    Ok(())
}
