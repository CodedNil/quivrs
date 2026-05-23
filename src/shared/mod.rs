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

    pub fn first_image(&self) -> Option<String> {
        self.sources.iter().find_map(|s| {
            s.images
                .first()
                .and_then(|img| img.split('|').next())
                .filter(|u| !u.is_empty())
                .map(str::to_string)
        })
    }

    pub fn thumbnail_image(&self) -> Option<String> {
        self.entry
            .as_ref()
            .map(|e| e.thumbnail.clone())
            .filter(|t| !t.is_empty())
            .or_else(|| self.first_image())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ArticleData {
    pub id: Uuid,
    pub status: ArticleStatus,
    pub rating: Option<Rating>,
    pub article: StoredArticle,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleSource {
    pub url: String,
    /// Domain for web articles, handle for social
    pub source: String,
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

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ArticleEntry {
    /// Article's title, kept concise and descriptive, max 8 words.
    pub title: String,

    /// URL for the article's thumbnail image.
    pub thumbnail: String,

    /// URL for the article's popout image.
    pub popout_image: String,

    /// Short informative summary, a few sentences max and no newlines.
    pub description: String,

    /// The article's content, written in HTML.
    pub content: String,

    /// A wiki-style sidebar for key facts, summaries, or metadata, written in HTML.
    pub sidebar: Option<String>,
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
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
#[cfg_attr(
    not(target_arch = "wasm32"),
    derive(sqlx::Type),
    sqlx(type_name = "TEXT")
)]
pub enum Category {
    Business,
    Politics,
    Law,
    Health,
    Culture,
    Lifestyle,
    Transport,
    Nature,
    Technology,
    Software,
    AI,
    Science,
    Sports,
    Gaming,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, EnumString,
)]
#[cfg_attr(
    not(target_arch = "wasm32"),
    derive(sqlx::Type),
    sqlx(type_name = "TEXT")
)]
pub enum ArticleStatus {
    Stored,
    New,
    Binned,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[cfg_attr(
    not(target_arch = "wasm32"),
    derive(sqlx::Type),
    sqlx(type_name = "TEXT")
)]
pub enum Rating {
    Hated,
    Disliked,
    Neutral,
    Liked,
    Loved,
}

impl Rating {
    pub const fn color(self) -> &'static str {
        match self {
            Self::Hated => "#873535",
            Self::Disliked => "#A3674E",
            Self::Neutral => "#494d64",
            Self::Liked => "#5D7D31",
            Self::Loved => "#3E6E2F",
        }
    }
}
