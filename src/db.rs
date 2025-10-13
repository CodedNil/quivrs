use anyhow::Result;
use bincode::config;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use rss::Channel;
use std::env;
use tokio::sync::OnceCell;

const FEEDS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("feeds");
static DB: OnceCell<Database> = OnceCell::const_new();

/// Ensure the `redb` database is ready for use.
pub async fn init() -> Result<()> {
    let db = database().await?;
    let write_txn = db.begin_write()?;
    {
        // Opening the table creates it if it doesn't exist.
        let _ = write_txn.open_table(FEEDS_TABLE)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Save or update a feed's channel snapshot.
pub async fn save_channel(feed_id: &str, channel: &Channel) -> Result<()> {
    let db = database().await?;
    let channel_bytes = bincode::serde::encode_to_vec(channel, config::standard())?;

    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(FEEDS_TABLE)?;
        table.insert(feed_id, channel_bytes.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Load every feed channel stored in the database.
pub async fn load_all_channels() -> Result<Vec<(String, Channel)>> {
    let db = database().await?;
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(FEEDS_TABLE)?;

    let mut channels = Vec::new();
    for entry in table.iter()? {
        let (id_ref, channel_bytes_ref) = entry?;
        let id = id_ref.value().to_string();
        let channel_bytes = channel_bytes_ref.value();
        let channel: Channel =
            bincode::serde::decode_from_slice(channel_bytes, config::standard())?.0;
        channels.push((id, channel));
    }
    Ok(channels)
}

/// Returns a reference to the global `redb::Database` instance, initializing it if necessary.
async fn database() -> Result<&'static Database> {
    DB.get_or_try_init(|| async {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "quivrs.redb".to_string());
        Database::create(database_url).map_err(anyhow::Error::from)
    })
    .await
}
