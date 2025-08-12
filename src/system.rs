//! System Module
//!
//! Handles communication with the Llama AI model API and configuration management.
//! Implements request/response structures and message handling functionality.
use config::{Config, ConfigError, File, FileFormat};

use reqwest::{
    Client,
    header::{self, HeaderMap},
};
use tracing::{Level, event};

use std::{path::Path, sync::Arc};

use crate::{
    CONFIG,
    lm_types::{Answer, Message},
    storage::Storage,
};

const CHUNK_SIZE: usize = 4095;

use once_cell::sync::Lazy;
use regex::Regex;

static THINK_TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)<think>.*?</think>").expect("valid regex"));

/// Loads configuration from settings.toml file
///
/// # Returns
/// * `Result<Config, ConfigError>` - Configuration object or error
pub fn get_config() -> Result<Config, ConfigError> {
    Config::builder()
        .add_source(File::from(Path::new("./settings.toml")).format(FileFormat::Toml))
        .build()
}

/// Sends a message to the Llama AI model and receives the response
///
/// # Arguments
/// * `context` - User message to be processed
/// * `user_id` - User identifier
/// * `storage` - Storage handler for conversation history
///
/// # Returns
/// * `String` - AI model response or error message
pub async fn reqwest_ai(context: String, user_id: i64, storage: Arc<dyn Storage>) -> Vec<String> {
    // Get configuration values with proper error handling
    let model = match CONFIG.get_string("model") {
        Ok(model) => model,
        Err(e) => {
            event!(Level::ERROR, "Configuration error: {}", e);
            return vec!["⚠️ Configuration error: Model not set".to_string()];
        }
    };

    let url = CONFIG.get_string("url").unwrap_or_else(|_| {
        event!(Level::WARN, "Using default API URL");
        "http://localhost:8080/v1/chat/completions".to_string()
    });

    // Add user message to conversation history
    storage
        .set_conversation_context(
            user_id,
            Message {
                role: "user".to_string(),
                content: context.clone(),
                reasoning: None,
            },
        )
        .await;

    // Prepare system context
    let fingerprint = storage.get_system_fingerprint(user_id).await;
    let temperature = storage.get_temperature(user_id).await;

    event!(
        Level::DEBUG,
        "System context: temp={}, fingerprint={}",
        temperature,
        fingerprint
    );

    // Build request headers
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

    if let Ok(api_key) = CONFIG.get_string("api_key") {
        if !api_key.is_empty() {
            headers.insert(
                header::AUTHORIZATION,
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
    }

    // Build message history
    let mut messages = vec![Message {
        role: "system".to_string(),
        content: fingerprint.clone(),
        reasoning: None,
    }];

    messages.extend(
        storage
            .list_notes(user_id)
            .await
            .iter()
            .map(|note| note.into()),
    );
    messages.extend(storage.get_conversation_context(user_id).await);

    // Prepare request body
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": temperature,
        "max_tokens": 2048,
        "stream": false
    });

    event!(Level::DEBUG, "Request body: {}", body.to_string());

    // Send request to AI service
    let client = Client::new();
    event!(Level::INFO, "Sending request to AI service");

    let response = match client.post(&url).headers(headers).json(&body).send().await {
        Ok(res) => res,
        Err(e) => {
            event!(Level::ERROR, "AI connection error: {}", e);
            return vec![format!("🔌 Connection error: {}", e)];
        }
    };

    // Process response
    let answer: Answer = match response.json().await {
        Ok(answer) => answer,
        Err(e) => {
            event!(Level::ERROR, "Invalid response format: {}", e);
            return vec!["❌ Invalid response from AI service".to_string()];
        }
    };

    event!(Level::INFO, "Received response from AI service");

    // Extract and clean AI response
    let ai_message = &answer.choices[0].message;
    let content = ai_message.content.as_str();

    // Apply thinking tag filter if configured
    let ret_message: Vec<char>;

    if !CONFIG.get_bool("thinking").unwrap_or(false) {
        ret_message = THINK_TAG_RE.replace_all(&content, "").chars().collect();
    } else {
        ret_message = content.chars().collect();
    }

    // Save AI response to conversation history
    storage
        .set_conversation_context(
            user_id,
            Message {
                role: "assistant".to_string(),
                content: content.to_string(),
                reasoning: None,
            },
        )
        .await;

    // Split content into Telegram-safe chunks
    let chunked_response = ret_message
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>();

    event!(
        Level::INFO,
        "Returning {} chunks for user {}",
        chunked_response.len(),
        user_id
    );

    chunked_response
}
