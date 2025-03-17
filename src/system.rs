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

use std::path::Path;

use crate::{
    lm_types::{Answer, Message},
    storage::{
        conversation::{get_history, update_or_insert_history},
        fingerprint::get_system_fingerprint,
        temperature::get_temperature,
    },
    CONFIG,
};

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
///
/// # Returns
/// * `String` - AI model response or error message
pub async fn send_message(context: String, user_id: i64) -> String {
    let client = Client::new();
    let url = CONFIG.get_string("url").unwrap_or(String::new());

    let model = CONFIG.get_string("model");
    if model.is_err() {
        return "Model not found".to_string();
    }

    // Get or create conversation history for user
    // And add user message to history
    update_or_insert_history(
        user_id,
        Message {
            role: "user".to_string(),
            content: context.clone(),
        },
    )
    .await;

    let temperature = get_temperature(user_id).await;

    let fingerprint = get_system_fingerprint(user_id).await;

    let mut messages = vec![Message {
        role: "system".to_string(),
        content: fingerprint.clone(),
    }];
    messages.extend(get_history(user_id).await);

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

    println!("{}: {}", chrono::Local::now(), "Лама печатает".green());
    let res = client.post(url).headers(header).json(&body).send().await;

    match res {
        Ok(res) => {
            let text = res.json::<Answer>().await;
            match text {
                Ok(text) => {
                    println!(
                        "{}: {}",
                        chrono::Local::now(),
                        "Llama return answer.".green()
                    );

                    update_or_insert_history(user_id, text.choices[0].message.clone()).await;

                    format!("{}", text.choices[0].message.content)
                }
                Err(e) => {
                    println!("{}{}", "Llama send wrong answer format: ".red(), e);
                    format!("Error with response: {}", e.to_string())
                }
            }
        }
        Err(e) => {
            println!("{}{}", "Llama connection error: ".red(), e);
            format!("Unable to connect: {}", e.to_string())
        }
    }
}
