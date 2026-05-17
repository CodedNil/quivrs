use super::StoredArticle;
use dioxus::prelude::*;

#[server]
#[allow(clippy::unused_async)]
pub async fn get_articles() -> Result<Vec<StoredArticle>, ServerFnError> {
    use crate::server::articles::{ARTICLES_TABLE, DB};
    use redb::{ReadableDatabase, ReadableTable};

    let read_txn = DB
        .begin_read()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let table = read_txn
        .open_table(ARTICLES_TABLE)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut articles: Vec<StoredArticle> = table
        .iter()
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .filter_map(std::result::Result::ok)
        .filter_map(|(_, v)| postcard::from_bytes::<StoredArticle>(v.value()).ok())
        .collect();

    articles.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(articles)
}
