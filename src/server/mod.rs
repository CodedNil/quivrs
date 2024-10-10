pub mod auth;
pub mod embeddings;
pub mod feed;

pub async fn init() {
    loop {
        feed::pull_articles().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(15 * 60)).await;
    }
}
