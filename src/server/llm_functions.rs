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
        "model": env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "google/gemini-3.1-flash-lite".to_string()),
        "service_tier": "flex",
        "provider": { "only": ["Google"] },
        "input": message,
        "text": {
            "format": {
                "type": "json_schema",
                "name": "output",
                "strict": true,
                "schema": schema_object
            },
            "verbosity": "low"
        },
        "reasoning": {
            "enabled": false
        },
        "tools": [
            {
                "type": "openrouter:web_search",
                "parameters": { "engine": "native" },
            }
        ]
    });

    // Write payload to payload.json
    let payload_str = serde_json::to_string_pretty(&payload).unwrap_or_default();
    std::fs::write("payload.json", payload_str).ok();

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

    // Write output to output.json
    let payload_str = serde_json::to_string_pretty(&response_json).unwrap_or_default();
    std::fs::write("output.json", payload_str).ok();

    // Scan through the output array to find the final output text
    let inner_text = response_json["output"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|step| step["content"].as_array())
        .flatten()
        .find(|item| item["type"] == "output_text")
        .and_then(|item| item["text"].as_str())
        .ok_or("Failed to extract output_text from response")?;

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
