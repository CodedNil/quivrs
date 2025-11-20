use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{env, error::Error};

use crate::feed::HTTP_CLIENT;

pub async fn run<T>(
    context: Vec<String>,
    message: &str,
) -> Result<T, Box<dyn Error + Send + Sync + 'static>>
where
    T: JsonSchema + DeserializeOwned,
{
    // Generate the JSON schema dynamically using `schemars`.
    let mut schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>();
    schema_object.remove("$schema");

    // Create the inputs
    let payload = json!({
        "model": "x-ai/grok-4.1-fast",
        "structured_outputs": true,
        "messages": [
            {
                "role": "system",
                "content": context.join("\n")
            },
            {
                "role": "user",
                "content": message
            }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "response",
                "strict": true,
                "schema": schema_object
            }
        },
        "reasoning": {
          "enabled": false
        }
    });

    // Send the request and check for errors
    let response = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions".to_string())
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

    // Parse response JSON and extract inner text.
    let response_json: Value = response.json().await?;
    let inner_text = response_json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or("Unexpected response structure")?;

    // If serialization fails, return an error including the inner text
    Ok(serde_json::from_str(inner_text)
        .map_err(|e| format!("Serialization failed: {e} - Outputted text: {inner_text}"))?)
}
