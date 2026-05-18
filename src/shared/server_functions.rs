use super::{ArticleData, ArticleStatus, Rating};
use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[server]
pub async fn get_user_articles() -> Result<Vec<ArticleData>, ServerFnError> {
    crate::server::database::get_user_articles().map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn set_article_status(id: Uuid, status: ArticleStatus) -> Result<(), ServerFnError> {
    crate::server::database::set_article_status(id, status)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn set_rating(id: Uuid, rating: Rating) -> Result<(), ServerFnError> {
    crate::server::database::set_rating(id, rating).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn get_all_item_ratings() -> Result<HashMap<String, Rating>, ServerFnError> {
    crate::server::database::get_all_item_ratings().map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn set_item_rating(key: String, rating: Rating) -> Result<(), ServerFnError> {
    crate::server::database::set_item_rating(&key, rating)
        .map_err(|e| ServerFnError::new(e.to_string()))
}
