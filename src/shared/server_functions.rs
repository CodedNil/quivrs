use super::StoredArticle;
use dioxus::prelude::*;

#[server]
#[allow(clippy::unused_async)]
pub async fn get_articles() -> Result<Vec<StoredArticle>, ServerFnError> {
    crate::server::database::get_all_articles().map_err(|e| ServerFnError::new(e.to_string()))
}
