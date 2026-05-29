#[allow(clippy::unused_async)]
pub mod server_functions;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use uuid::Uuid;

#[cfg(feature = "server")]
use surrealdb::types::SurrealValue;

#[cfg_attr(feature = "server", derive(SurrealValue))]
#[derive(Deserialize, Default)]
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
    pub region: Region,
    pub sentiment: f32,
    pub importance: f32,

    pub embedding: Vec<f32>,
    pub embedding_text: String,
    pub embedding_model: String,
}

#[cfg_attr(feature = "server", derive(SurrealValue))]
#[derive(Serialize, Deserialize)]
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
    pub region: Region,

    pub status: ArticleStatus,
    pub binned_at: Option<DateTime<Utc>>,
    pub rating: Option<Rating>,

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

#[cfg_attr(feature = "server", derive(SurrealValue))]
#[derive(Serialize, Deserialize)]
pub struct ArticleSource {
    pub url: String,
    pub domain: String,
}

#[cfg_attr(feature = "server", derive(SurrealValue))]
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

#[cfg_attr(feature = "server", derive(SurrealValue))]
#[derive(
    Serialize, Deserialize, Default, Clone, Copy, Display, EnumIter, EnumString, PartialEq, Eq,
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
    India,
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

#[cfg_attr(feature = "server", derive(SurrealValue))]
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
pub enum ArticleStatus {
    Stored,
    #[default]
    New,
    Binned,
}

#[cfg_attr(feature = "server", derive(SurrealValue))]
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
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
