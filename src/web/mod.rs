use crate::shared::StoredArticle;
use dioxus::prelude::*;

#[server]
async fn get_articles() -> Result<Vec<StoredArticle>, ServerFnError> {
    use crate::server::feed::{ARTICLES_TABLE, DB};
    use redb::ReadableDatabase;

    let read_txn = DB
        .begin_read()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let table = read_txn
        .open_table(ARTICLES_TABLE)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut articles = Vec::new();
    let iter = redb::ReadableTable::iter(&table).map_err(|e| ServerFnError::new(e.to_string()))?;

    for item in iter {
        let (_, v) = item.map_err(|e| ServerFnError::new(e.to_string()))?;
        if let Ok(article) = postcard::from_bytes::<StoredArticle>(v.value()) {
            articles.push(article);
        }
    }

    // Sort by updated_at descending
    articles.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(articles)
}

pub fn app() -> Element {
    let articles = use_resource(get_articles);

    rsx! {
        document::Title { "Quivrs" }
        document::Meta { name: "darkreader-lock" }
        div {
            h1 { "Quivrs" }

            match &*articles.read_unchecked() {
                Some(Ok(list)) => rsx! {
                    div {
                        for article in list {
                            article_card { key: "{article.id}", article: article.clone() }
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { "Error loading articles: {e}" }
                },
                None => rsx! {
                    div { "Loading articles..." }
                },
            }
        }
    }
}

#[component]
fn article_card(article: StoredArticle) -> Element {
    let updated = article.updated_at.format("%Y-%m-%d %H:%M").to_string();
    rsx! {
        div {
            h2 { "title: {article.entry.title}" }
            p { "description: {article.entry.description}" }
            div { dangerous_inner_html: "{article.entry.full_content}" }
            div {
                for source in &article.sources {
                    span { "#{source.url}" }
                }
            }
            div {
                for tag in &article.entry.tags {
                    span { "#{tag}" }
                }
            }
            div { "Updated: {updated}" }
        }
    }
}
