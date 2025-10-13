use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{env, error::Error, sync::LazyLock};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

pub async fn run<T>(
    context: Vec<String>,
    message: String,
) -> Result<T, Box<dyn Error + Send + Sync + 'static>>
where
    T: JsonSchema + DeserializeOwned,
{
    // Set up request headers.
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "Bearer {}",
            env::var("OPENROUTER").expect("OPENROUTER not set")
        ))?,
    );

    // Generate the JSON schema dynamically using `schemars`.
    let mut schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>();
    schema_object.remove("$schema");

    // Create the inputs
    let payload = json!({
        "model": "google/gemini-2.5-flash-lite-preview-09-2025",
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
                "name": "weather",
                "strict": true,
                "schema": schema_object
            }
        }
    });

    // Send the request and check for errors
    let response = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions".to_string())
        .headers(headers)
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

    Ok(serde_json::from_str(inner_text)?)
}
