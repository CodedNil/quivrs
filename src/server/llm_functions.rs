use super::HTTP_CLIENT;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{env, error::Error};
use tracing::info;

pub async fn run<T>(message: &str) -> Result<T, Box<dyn Error + Send + Sync>>
where
    T: DeserializeOwned,
{
    let payload = json!({
        "model": env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "deepseek/deepseek-v4-flash".to_string()),
        "provider": { "only": ["siliconflow/fp8", "atlas-cloud/fp8"] },
        "input": message,
        "reasoning": {
            "enabled": true
        },
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

    // Get cost of the prompt
    let cost = response_json["usage"]["cost"]
        .as_f64()
        .ok_or("Failed to extract cost from response")?;
    info!(
        "[GENERATION] '{}' - cost: ${}",
        inner_text
            .replace('\n', " ")
            .chars()
            .take(40)
            .collect::<String>(),
        cost,
    );

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
