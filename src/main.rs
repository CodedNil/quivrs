mod feed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        feed::pull_articles().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(15 * 60)).await;
    }
}
