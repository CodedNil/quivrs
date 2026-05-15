use super::HTTP_CLIENT;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{env, error::Error};
use tap::Tap;

pub async fn run<T>(message: &str) -> Result<T, Box<dyn Error + Send + Sync>>
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
        "model": env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "deepseek/deepseek-v4-pro".to_string()),
        "input": message,
        "text": {
            "format": {
                "type": "json_schema",
                "name": std::any::type_name::<T>(),
                "strict": true,
                "schema": schema_object
            },
            "verbosity": "low"
        },
        "reasoning": {
            "enabled": false
        }
    });

    let response = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/responses")
        .header(CONTENT_TYPE, "application/json")
        .header(
            AUTHORIZATION,
            format!(
                "Bearer {}",
                env::var("OPENROUTER").expect("OPENROUTER not set")
            ),
        )
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!(
            "Request failed {}: {}",
            response.status(),
            response.text().await?
        )
        .into());
    }

    let response_json: Value = response.json().await?;
    let inner_text = response_json
        .pointer("/output/0/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or("Unexpected response structure")?;

    serde_json::from_str(inner_text).map_err(|e| {
        format!(
            "Serialization failed: {e} - Output received: {}",
            inner_text
                .replace('\n', " ")
                .chars()
                .take(500)
                .collect::<String>()
        )
        .into()
    })
}
