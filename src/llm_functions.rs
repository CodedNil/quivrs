use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
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
    // Construct the URL with proper variable substitution
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}",
        model = "gemini-2.0-flash",
        api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set")
    );

    // Set up request headers.
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Generate the JSON schema dynamically using `schemars`.
    let mut schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>();
    schema_object.remove("$schema");

    // Create the inputs
    let mut payload = json!({
        "contents": [{"parts": [{"text": message}]}],
        "generationConfig": {
            "response_mime_type": "application/json",
            "response_schema": schema_object
        }
    });
    if !context.is_empty() {
        let system_parts = context
            .into_iter()
            .map(|msg| json!({"text": msg}))
            .collect::<Vec<_>>();
        payload["system_instruction"] = json!({"parts": system_parts});
    }

    // Send the request and check for errors
    let response = HTTP_CLIENT
        .post(url)
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
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .ok_or("Unexpected response structure")?;

    Ok(serde_json::from_str(inner_text)?)
}
