use crate::feed::HTTP_CLIENT;
use anyhow::Result;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{env, error::Error};
use tap::Tap;

pub async fn run<T>(context: &str, message: &str) -> Result<T, Box<dyn Error + Send + Sync>>
where
    T: JsonSchema + DeserializeOwned,
{
    let schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>()
    .tap_mut(|s| {
        s.remove("$schema");
    });

    let payload = json!({
        "model": env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "deepseek/deepseek-v4-flash".to_string()),
        "structured_outputs": true,
        "messages": [
            { "role": "system", "content": context },
            { "role": "user",   "content": message }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "schema",
                "strict": true,
                "schema": schema_object
            }
        }
    });

    let response = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header(CONTENT_TYPE, "application/json")
        .header(
            AUTHORIZATION,
            &format!(
                "Bearer {}",
                env::var("OPENROUTER").expect("OPENROUTER not set")
            ),
        )
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!(
            "Request failed with status {}: {}",
            response.status(),
            response.text().await?
        )
        .into());
    }

    let response_json: Value = response.json().await?;
    let inner_text = response_json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or("Unexpected response structure")?;

    serde_json::from_str(inner_text)
        .map_err(|e| format!("Serialization failed: {e} - Outputted text: {inner_text}").into())
}
