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

#[derive(Serialize, Deserialize, Clone, Eq)]
pub struct ArticleSource {
    pub url: String,
    pub title: String,
    pub summary: String,
    pub image: Option<String>,
    pub image_description: Option<String>,
    pub published: DateTime<Utc>,
}

impl PartialEq for ArticleSource {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

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

    /// Descriptive tags, lowercase and hyphen-separated (e.g. technology, ai, machine-learning).
    pub tags: Vec<String>,

    pub article_type: ArticleType,
    pub category: Category,
    /// Whether the article is sponsored and the authors are receiving compensation. Including if the article is heavily associated with affiliate links.
    pub sponsored: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum Section {
    /// Header for a section that follows, add before every instance of List.
    Header(String),

    /// Paragraph of text. Supports inline emphasis: *bold* and _italic_.
    /// Multiple paragraphs are recommended when there is a lot of content, cover it in depth without repeating yourself.
    Paragraph(String),

    /// Full width image with caption. "url~caption", include multiple of these throughout the article.
    /// Try to include all the images available.
    Image(String),

    /// Simple bulleted list of items. Can have a title if written as "title~text" or just text.
    ///
    /// Example usages:
    /// - A timeline of events, history leading up to this article. "date~text" where date is the header and is for example "2026" or "January 2026" or "13th January 2026". Text should be max one sentence.
    List(Vec<String>),

    /// Same as List but each item is rendered inside a bordered box, stacked vertically.
    /// Items can use "title~text" (title shown as small label above the text) or plain text.
    ///
    /// Example usages:
    /// - Up to 4 short highlights that give an overview of this article and key details. "title~text"
    RowBoxes(Vec<String>),

    /// Same as List but each item is rendered inside a bordered box, laid out horizontally. Max 4 items for readability.
    /// Items can use "title~text" (title shown as small label above the text) or plain text.
    ///
    /// Example usages:
    /// - Up to 4 different perspectives on this article. Each formatted as a string "name~text". Name is the person or organization that is presenting this perspective, could also be a viewpoint like "Skeptics" that provides a critical view.
    ColumnBoxes(Vec<String>),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum ArticleType {
    BreakingNews,
    News,
    Opinion,

    Marketing,
    Sale,
    Review,

    Blog,
    Newsletter,
    Video,
    Post,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum Category {
    World,
    Business,
    Culture,
    Politics,
    Health,
    Technology,
    Science,
    Education,
    Sports,
    Gaming,
}
