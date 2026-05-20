#[allow(clippy::unused_async)]
pub mod server_functions;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredArticle {
    pub id: Uuid,
    /// List of sources used to generate the article.
    pub sources: Vec<ArticleSource>,
    /// Estimated user interest 0.0-1.0
    pub estimated_liked: f32,

    /// Generated content for the article
    pub entry: Option<ArticleEntry>,

    pub category: Category,

    /// Timestamp when the first source was published.
    pub published: DateTime<Utc>,
    /// Timestamp when the article was last updated.
    pub updated_at: DateTime<Utc>,
}

impl PartialEq for StoredArticle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl StoredArticle {
    pub fn display_title(&self) -> &str {
        self.entry.as_ref().map_or_else(
            || self.sources.first().map_or("", |s| s.title.as_str()),
            |e| e.title.as_str(),
        )
    }

    pub fn display_description(&self) -> &str {
        self.entry.as_ref().map_or_else(
            || self.sources.first().map_or("", |s| s.summary.as_str()),
            |e| e.description.as_str(),
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArticleData {
    pub id: Uuid,
    pub status: ArticleStatus,
    pub rating: Option<Rating>,
    pub article: StoredArticle,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArticleSource {
    pub url: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    /// Each entry is `"url|caption"` — split on `|` to get the two parts.
    pub images: Vec<String>,
    pub published: DateTime<Utc>,
}

impl PartialEq for ArticleSource {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for ArticleSource {}
impl std::hash::Hash for ArticleSource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct ArticleEntry {
    /// Article's title, kept concise and descriptive, max 8 words.
    pub title: String,

    /// Short informative summary, a few sentences max and no newlines.
    pub description: String,

    /// The article's content, broken down into sections, first should always be a paragraph and second an image if there is one available.
    /// All the important information should be in the first few sections, with the rest being a deeper dive/expansion.
    /// You can and should use sections multiple times throughout the article, multiple paragraphs and multiple images.
    /// Often include Highlights and Perspectives (Boxes with Header)
    pub sections: Vec<Section>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum Section {
    /// Header for a section that follows, add before every instance of List.
    Header(String),

    /// Paragraph of text. Supports inline emphasis: *bold* and _italic_.
    /// Multiple paragraphs are recommended when there is a lot of content, cover it in depth without repeating yourself.
    Paragraph(String),

    /// Full width image with caption. "url|caption", include multiple of these throughout the article.
    /// Try to include all the images available. Favour the highest resolution images available.
    Image(String),

    /// Simple bulleted list of items. Can have a title if written as "title|text" or just text.
    ///
    /// Example usages:
    /// - A timeline of events, history leading up to this article. "date|text" where date is the header and is for example "2026" or "January 2026" or "13th January 2026". Text should be max one sentence.
    List(Vec<String>),

    /// Same as List but each item is rendered inside a bordered box, stacked vertically.
    /// Items can use "title|text" (title shown as small label above the text) or plain text.
    ///
    /// Example usages:
    /// - Up to 4 short highlights that give an overview of this article and key details. "title|text"
    RowBoxes(Vec<String>),

    /// Same as List but each item is rendered inside a bordered box, laid out horizontally. Max 4 items for readability.
    /// Items can use "title|text" (title shown as small label above the text) or plain text.
    ///
    /// Example usages:
    /// - Up to 4 different perspectives on this article. Each formatted as a string "name|text". Name is the person or organization that is presenting this perspective, could also be a viewpoint like "Skeptics" that provides a critical view.
    ColumnBoxes(Vec<String>),
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Display,
    EnumIter,
    EnumString,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub enum Category {
    Business,
    Politics,
    Law,
    Health,
    Culture,
    Lifestyle,
    Environment,
    Technology,
    Science,
    Sports,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum ArticleStatus {
    New,
    Stored,
    Binned,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum Rating {
    Hated,
    Disliked,
    Neutral,
    Liked,
    Loved,
}
