#[allow(clippy::unused_async)]
pub mod server_functions;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use uuid::Uuid;

#[derive(Clone)]
pub struct PendingSource {
    pub url: String,

    pub domain: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub images: Vec<(String, String)>, // (url, caption)
    pub published: DateTime<Utc>,
    pub category: Category,

    pub embedding: Vec<f32>,
    pub embedding_text: String,
    pub embedding_model: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Article {
    pub id: Uuid,

    pub sources: Vec<ArticleSource>,
    pub title: String,
    pub description: String,
    pub content: String,
    pub sidebar: String,
    pub thumbnail: String,
    pub published: DateTime<Utc>,
    pub category: Category,

    pub status: ArticleStatus,
    pub binned_at: Option<DateTime<Utc>>,
    pub rating: Option<Rating>,

    pub embedding: Vec<f32>,
    pub embedding_text: String,
    pub embedding_model: String,
}

impl PartialEq for Article {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleSource {
    pub url: String,
    pub domain: String,
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
    Hash,
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
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
