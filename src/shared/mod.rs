pub mod server_functions;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredArticle {
    pub id: Uuid,
    pub sources: Vec<ArticleSource>,
    /// Estimated user interest 0.0-1.0
    pub estimated_liked: f32,
    pub entry: Option<ArticleEntry>,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PartialEq for StoredArticle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArticleSource {
    pub url: String,
    pub title: String,
    pub description: String,
    pub image: Option<(String, Option<String>)>,
    pub published: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, JsonSchema, Default, Clone)]
pub struct ArticleEntry {
    /// Article's title, kept concise and descriptive, max 8 words.
    pub title: String,

    /// Short informative summary, a few sentences max and no newlines.
    pub description: String,

    /// The article's content, broken down into sections, first should always be a paragraph and second an image if there is one available.
    /// You can and should use sections multiple times throughout the article, multiple paragraphs and multiple images.
    /// Multiple paragraphs are recommended when there is a lot of content, cover it in depth without repeating yourself.
    /// Try to include all the images available.
    pub sections: Vec<Section>,

    /// Descriptive tags, lowercase and hyphen-separated (e.g. technology, ai, machine-learning).
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum Section {
    /// Header for a section that follows, add before every instance of List.
    Header(String),

    /// Paragraph of text.
    Paragraph(String),

    /// Full width image with caption. "url~caption", include multiple of these throughout the article.
    Image(String),

    /// Simple bulleted list of items.
    List(Vec<String>),
}
