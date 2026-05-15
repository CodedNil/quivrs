pub mod server_functions;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StoredArticle {
    pub id: Uuid,
    pub sources: Vec<ArticleSource>,
    /// Estimated user interest 0.0-1.0
    pub estimated_liked: f32,
    pub entry: ArticleEntry,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, JsonSchema, Default, Clone, Debug, PartialEq, Eq)]
pub struct ArticleEntry {
    /// Article's title, kept concise and descriptive, max 8 words
    pub title: String,
    /// Short informative summary, a few sentences max and no newlines
    pub description: String,
    /// Full article content as rendered HTML; use figure/figcaption for images, include inline links, well-structured paragraphs; place first image after first paragraph as thumbnail
    pub full_content: String,
    /// Descriptive tags, lowercase and hyphen-separated (e.g. technology, ai, machine-learning)
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArticleSource {
    pub url: String,
    pub title: String,
    pub description: String,
    pub published: DateTime<Utc>,
}
