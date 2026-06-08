#[allow(clippy::unused_async)]
pub mod server_functions;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default)]
pub struct PendingSource {
    pub url: String,

    pub domain: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub images: Vec<CaptionedImage>,
    pub published: DateTime<Utc>,
    pub category: Category,
    pub region: Region,
    pub sentiment: f32,
    pub importance: f32,

    pub embedding: Vec<f32>,
    pub embedding_text: String,
    pub embedding_model: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CaptionedImage {
    pub url: String,
    pub caption: String,
}

#[derive(Serialize, Deserialize)]
pub struct Article {
    pub id: Uuid,

    pub sources: Vec<ArticleSource>,
    pub title: String,
    pub description: String,
    pub content: String,
    pub sidebar: String,
    pub thumbnail: String,
    pub background: String,
    pub published: DateTime<Utc>,
    pub category: Category,
    pub region: Region,

    pub status: ArticleStatus,
    pub status_changed: DateTime<Utc>,
    pub rating: Option<Rating>,
    pub estimated_liked: Option<f32>,

    pub embedding: Vec<f32>,
    pub embedding_text: String,
    pub embedding_model: String,

    pub sentiment: f32,
    pub importance: f32,
}

impl PartialEq for Article {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize)]
pub struct ArticleSource {
    pub url: String,
    pub domain: String,
}

#[derive(
    Serialize,
    Deserialize,
    Default,
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
pub enum Category {
    #[default]
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
    Serialize, Deserialize, Default, Clone, Copy, Display, EnumIter, EnumString, PartialEq, Eq, Hash,
)]
pub enum Region {
    #[default]
    Global,

    // Great Britain
    UnitedKingdom,
    England,
    Scotland,
    Wales,
    Ireland,

    // North America
    UnitedStates,
    Canada,

    // Latin America and the Caribbean
    LatinAmerica,
    Caribbean,

    // Europe
    France,
    Germany,
    Italy,
    Iberia,
    Nordic,
    WesternEurope,
    CentralEurope,
    Balkans,
    EasternEurope,

    // Middle East and Africa
    MiddleEastNorthAfrica,
    SubSaharanAfrica,

    // Asia
    IndianSubcontinent,
    WestAsia,
    China,
    Japan,
    Korea,
    Taiwan,
    SoutheastAsia,
    CentralAsia,

    // Oceania
    Oceania,
}

#[derive(
    Serialize,
    Deserialize,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Display,
    EnumString,
)]
pub enum ArticleStatus {
    Stored,
    #[default]
    New,
    Binned,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum Rating {
    Hated,
    Disliked,
    #[default]
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
