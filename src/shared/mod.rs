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

    /// Sources for the article, "url~title".
    pub sources: Vec<String>,

    /// Descriptive tags, lowercase and hyphen-separated (e.g. technology, ai, machine-learning).
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum Section {
    /// Paragraph of text.
    Text(String),

    /// Full width image with caption. "url~caption", include multiple of these throughout the article.
    Image(String),

    /// Two images side by side with captions. "url~caption|url~caption".
    TwoImages(String),

    /// Up to 4 short highlights that give an overview of this article and key details.
    /// Each formatted as a string "header~text".
    Highlights(Vec<String>),

    /// Up to 4 different perspectives on this article. Each formatted as a string "header~text".
    /// Header is the person or organization that is presenting this perspective, could also be a viewpoint like "Skeptics" that provides a critical view.
    Perspectives(Vec<String>),

    /// Background information including timeline, if available/relevant.
    /// Should only include this section if it's important and informative for the historical context.
    Background(Background),

    /// Pros and Cons informative if it is a review article.
    /// Only include if there are multiple of each.
    ProsCons(ProsCons),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct Background {
    /// Short summary of the background, can be left blank if not relevant.
    pub text: String,
    /// Timeline of events, can be left empty if not relevant. "date~text" where date is the header and is for example "2026" or "January 2026" or "13th January 2026". Text should be max one sentence.
    pub timeline: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct ProsCons {
    /// Pros of the article, comma separated, sentence-case.
    pub pros: String,
    /// Cons of the article, comma separated, sentence-case.
    pub cons: String,
}
