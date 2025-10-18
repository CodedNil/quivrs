use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use redb::ReadableDatabase;
use reqwest::header::CONTENT_TYPE;
use std::error::Error;
use tracing::warn;

use crate::feed::{DB, FEEDS_TABLE, FeedData};

/// A custom error type to centralize error handling and response generation.
pub enum AppError {
    NotFound(String),
    Internal(Box<dyn Error + Send + Sync>),
}

/// Converts `AppError` into an HTTP response, handling status codes and logging.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Self::Internal(e) => {
                warn!("Internal server error: {e:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}

/// Allows any standard error to be converted into an `AppError`.
impl<E> From<E> for AppError
where
    E: Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        Self::Internal(Box::new(err))
    }
}

/// Axum handler to serve the generated feed.
pub async fn summarised_feed_handler(
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let read_txn = DB.begin_read()?;
    let table = read_txn.open_table(FEEDS_TABLE)?;
    let feed_bytes_guard = table
        .get(id.as_str())?
        .ok_or_else(|| AppError::NotFound(format!("Feed with ID '{id}' not found")))?;

    let feed_data: FeedData = postcard::from_bytes(feed_bytes_guard.value())?;
    let feed_string = serde_json::to_string(&feed_data.to_json_feed())?;

    Ok((
        StatusCode::OK,
        [(CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        feed_string,
    ))
}
