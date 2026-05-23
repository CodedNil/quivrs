use super::{ArticleData, ArticleStatus, Rating};
use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[server]
pub async fn get_user_articles() -> Result<Vec<ArticleData>, ServerFnError> {
    crate::server::database::get_user_articles()
        .await
        .map_err(|e| {
            error!("Failed to get user articles: {}", e);
            ServerFnError::new(e.to_string())
        })
}

#[server]
pub async fn set_article_status(id: Uuid, status: ArticleStatus) -> Result<(), ServerFnError> {
    crate::server::database::set_article_status(id, status)
        .await
        .map_err(|e| {
            error!("Failed to set article status: {}", e);
            ServerFnError::new(e.to_string())
        })
}

#[server]
pub async fn set_rating(id: Uuid, rating: Rating) -> Result<(), ServerFnError> {
    crate::server::database::set_rating(id, rating)
        .await
        .map_err(|e| {
            error!("Failed to set rating: {}", e);
            ServerFnError::new(e.to_string())
        })
}

#[server]
pub async fn get_all_item_ratings() -> Result<HashMap<String, Rating>, ServerFnError> {
    crate::server::database::get_all_item_ratings()
        .await
        .map_err(|e| {
            error!("Failed to get all item ratings: {}", e);
            ServerFnError::new(e.to_string())
        })
}

#[server]
pub async fn set_item_rating(key: String, rating: Rating) -> Result<(), ServerFnError> {
    crate::server::database::set_item_rating(&key, rating)
        .await
        .map_err(|e| {
            error!("Failed to set item rating: {}", e);
            ServerFnError::new(e.to_string())
        })
}

#[server]
pub async fn reclassify_articles(ids: Vec<Uuid>) -> Result<(), ServerFnError> {
    crate::server::database::reclassify_articles(ids)
        .await
        .map_err(|e| {
            error!("Failed to reclassify articles: {}", e);
            ServerFnError::new(e.to_string())
        })
}

#[server]
pub async fn regenerate_article(id: Uuid) -> Result<(), ServerFnError> {
    crate::server::database::regenerate_article(id)
        .await
        .map_err(|e| {
            error!("Failed to regenerate article {}", e);
            ServerFnError::new(e.to_string())
        })
}
