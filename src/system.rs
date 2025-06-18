//! System Module
//!
//! Handles communication with the Llama AI model API and configuration management.
//! Implements request/response structures and message handling functionality.
use colored::Colorize;
use config::{Config, ConfigError, File, FileFormat};

use reqwest::{
    header::{self, HeaderMap},
    Client,
};

use std::{path::Path, sync::Arc};

use crate::{
    lm_types::{Answer, Message},
    storage::Storage,
    CONFIG,
};

const CHUNK_SIZE: usize = 4095;

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
pub async fn reqwest_ai(
    context: String,
    user_id: i64,
    storage: Arc<dyn Storage>,
) -> Vec<String> {
    let client = Client::new();
    let url = CONFIG.get_string("url").unwrap_or(String::new());

    let model = CONFIG.get_string("model");
    if model.is_err() {
        return vec!["Model not found".to_string()];
    }

    // Get or create conversation history for user
    // And add user message to history
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

    let temperature = storage.get_temperature(user_id).await;

    let fingerprint = storage.get_system_fingerprint(user_id).await;

    let mut messages = vec![Message {
        role: "system".to_string(),
        content: fingerprint.clone(),
        reasoning: None,
    }];

    messages.extend(storage.get_conversation_context(user_id).await);

    let mut header = HeaderMap::new();
    header.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

    print!("temperature: {}, system: {} ", temperature, fingerprint);

    let body = serde_json::json!({
        "model": model.unwrap(),
        "messages": messages,
        "temperature": temperature,
        "max_tokens": -1,
        "stream": false
    });

    println!("{}: {}", chrono::Local::now(), "AI writing".green());
    let res = client.post(url).headers(header).json(&body).send().await;

    match res {
        Ok(res) => {
            let text = res.json::<Answer>().await;
            match text {
                Ok(text) => {
                    // println!("Answer: {:?}", text);
                    println!("{}: {}", chrono::Local::now(), "AI return answer.".green());

                    storage
                        .set_conversation_context(user_id, text.choices[0].message.clone())
                        .await;

                    let ret_message: Vec<char> = text.choices[0].message.content.chars().collect();
                    let mut ret_vec: Vec<String> = vec![];

                    for chunk in ret_message.chunks(CHUNK_SIZE) {
                        ret_vec.push(format!("{}", chunk.iter().collect::<String>()));
                    }

                    ret_vec
                }
                Err(e) => {
                    println!("{}{}", "Llama send wrong answer format: ".red(), e);
                    vec![format!("Error with response: {}", e.to_string())]
                }
            }
        }
        Err(e) => {
            println!("{}{}", "Llama connection error: ".red(), e);
            vec![format!("Unable to connect: {}", e.to_string())]
        }
    }
}

#[allow(dead_code)]
fn replace_escape_sequences(input: &str) -> String {
    input
        // .replace("*", "\\*")
        .replace(".", "\\.")
        .replace("-", "\\-")
        // .replace(">", "\\>")
        // .replace("_", "\\_")
        .replace("/", "\\/")
        .replace("^", "\\^")
        .replace("=", "\\=")
        .replace("!", "\\!")
        .replace("|", "\\|")
        .replace("#", "\\#")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("{", "\\{")
        .replace("}", "\\}")
}
