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
use tracing::{event, Level};

use std::{path::Path, sync::Arc};

use crate::{
    lm_types::{Answer, Message},
    storage::Storage,
    CONFIG,
};

const CHUNK_SIZE: usize = 4095;

use once_cell::sync::Lazy;
use regex::Regex;

static THINK_TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)<think>.*?</think>").unwrap());

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
    is_maid: bool,
) -> Vec<String> {
    let client = Client::new();
    let url = CONFIG.get_string("url").unwrap_or(String::new());

    let model = CONFIG.get_string("model");
    if model.is_err() {
        event!(Level::ERROR, "Model not found");
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

    let fingerprint: String = storage.get_system_fingerprint(user_id).await;
    

    let mut messages = vec![Message {
        role: "system".to_string(),
        content: fingerprint.clone(),
        reasoning: None,
    }];

    if !is_maid {
        messages.extend(storage.get_conversation_context(user_id).await);
    }

    let mut header = HeaderMap::new();
    header.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

    let api_key = CONFIG.get_string("api_key");

    if let Ok(key) = api_key {
        if !key.eq("") {
            header.insert(header::AUTHORIZATION, format!("Bearer {key}").parse().unwrap());
        }
    }

    event!(
        Level::INFO,
        "temperature: {}, system: {} ",
        temperature,
        fingerprint
    );

    let body = serde_json::json!({
        "model": model.unwrap(),
        "messages": messages,
        "temperature": temperature,
        "max_tokens": 2048,
        "stream": false
    });

    event!(
        Level::INFO,
        "{}: {}",
        chrono::Local::now(),
        "AI writing".green()
    );
    let res = client.post(url).headers(header).json(&body).send().await;
    event!(Level::INFO, "returned result {:?}", res);
    match res {
        Ok(res) => {
            let text = res.json::<Answer>().await;
            event!(Level::INFO, "returned text {:?}", text);
            match text {
                Ok(text) => {
                    // println!("Answer: {:?}", text);
                    event!(
                        Level::INFO,
                        "{}: {}",
                        chrono::Local::now(),
                        "AI return answer.".green()
                    );

                    let message = text.choices[0].message.clone();

                    storage
                        .set_conversation_context(user_id, message.clone())
                        .await;

                    let ret_message: Vec<char>;

                    if !CONFIG.get_bool("thinking").unwrap_or(false) {
                        ret_message = THINK_TAG_RE
                            .replace_all(&message.content, "")
                            .chars()
                            .collect();
                        println!("ret_mess1");
                    } else {
                        ret_message = message.content.chars().collect();
                        println!("ret_mess2");
                    }
                    let mut ret_vec: Vec<String> = vec![];

                    for chunk in ret_message.chunks(CHUNK_SIZE) {
                        ret_vec.push(format!("{}", chunk.iter().collect::<String>()));
                    }

                    ret_vec
                }
                Err(e) => {
                    event!(
                        Level::INFO,
                        "{}{}",
                        "Llama send wrong answer format: ".red(),
                        e
                    );
                    vec![format!("Error with response: {}", e.to_string())]
                }
            }
        }
        Err(e) => {
            event!(Level::INFO, "{}{}", "Llama connection error: ".red(), e);
            vec![format!("Unable to connect: {}", e.to_string())]
        }
    }
}
