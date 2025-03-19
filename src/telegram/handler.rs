//! Command Handler Module
//!
//! This module implements the telegram bot command handling functionality.
//! It processes user commands and manages interactions with the Llama AI model.
use crate::{storage::Storage, system};
use std::{collections::HashSet, sync::Arc};
use teloxide::{prelude::*, types::Message, utils::command::BotCommands, Bot};
use tokio::sync::Mutex;

/// Bot commands enumeration
///
/// Defines all available bot commands with their descriptions.
/// Uses lowercase naming convention for command matching.
#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    /// Starts the bot and displays help message
    #[command(description = "начало работы с ботом. Для получения списка команд напишите /help")]
    Start,
    /// Displays list of available commands
    #[command(description = "отображение команд.")]
    Help,
    /// Processes chat requests with Llama AI
    /// Takes a String parameter containing the user's prompt
    #[command(
        description = "напишите текст запроса после команды. Он будет использован как promt для запроса."
    )]
    Chat(String),
    /// Clears conversation history
    #[command(description = "очищает контекст диалога.")]
    Clear,
    /// Sets system fingerprint for the model
    #[command(
        description = "устанавливает системный fingerprint для модели. Для установки напишите сообщение с системным указанием модели."
    )]
    System(String),
    /// Sets temperature for the model
    #[command(
        description = "устанавливает температуру для модели. Для установки укажите температуру от 0.0 до 1.0."
    )]
    Temperature(f32),
    /// Stops current operation
    #[command(description = "остановить текущую операцию.")]
    Stop,
}

/// Main command handler function
///
/// Processes incoming bot commands and returns appropriate responses
///
/// # Arguments
/// * `bot` - Telegram Bot instance
/// * `msg` - Incoming message containing the command
/// * `command` - Parsed command enum
/// * `senders` - Thread-safe set of chat IDs who await for the answer
///
/// # Returns
/// * `ResponseResult<()>` - Result of the command execution
pub async fn answer(
    bot: Bot,
    msg: Message,
    command: Command,
    senders: Arc<Mutex<HashSet<i64>>>,
    storage: Arc<Mutex<Box<dyn Storage>>>,
) -> ResponseResult<()> {
    match command {
        Command::Start => {
            bot.send_message(msg.chat.id, "Welcome to Llama AI Telegram Bot!")
                .await?;
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Chat(text) => {
            if senders.lock().await.contains(&msg.chat.id.0) {
                bot.send_message(msg.chat.id, "Please wait...").await?;
                bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                    .await?;
            } else {
                senders.lock().await.insert(msg.chat.id.0);
                let bot_clone = bot.clone();
                let chat_id = msg.chat.id;
                let senders_clone = senders.clone();
                let storage_clone = storage.clone();
                bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                    .await?;

                tokio::spawn(async move {
                    let answer = system::send_message(text, chat_id.0, storage_clone).await;
                    if let Err(e) = bot_clone.send_message(chat_id, answer).await {
                        println!("Failed to send message: {}", e);
                    }
                    senders_clone.lock().await.remove(&chat_id.0);
                });
            }
        }
        Command::System(fingerprint) => {
            storage
                .lock()
                .await
                .set_system_fingerprint(msg.chat.id.0, fingerprint)
                .await;
            bot.send_message(msg.chat.id, "System fingerprint set")
                .await?;
        }
        Command::Temperature(temperature) => {
            storage
                .lock()
                .await
                .set_temperature(msg.chat.id.0, temperature)
                .await;
            bot.send_message(msg.chat.id, "Temperature set").await?;
        }
        Command::Clear => {
            storage
                .lock()
                .await
                .clear_conversation_context(msg.chat.id.0)
                .await;
            bot.send_message(msg.chat.id, "Conversation cleared")
                .await?;
        }
        Command::Stop => {
            bot.send_message(msg.chat.id, "Stop").await?;
        }
    };
    Ok(())
}

/// Message handler
/// Alternative of /chat command
///
/// # Arguments
/// * `bot` - Telegram Bot instance
/// * `msg` - Incoming message containing the command
/// * `senders` - Thread-safe set of chat IDs who await for the answer
///
/// # Returns
/// * `ResponseResult<()>` - Result of the command execution
pub async fn message_handler(
    bot: Bot,
    msg: Message,
    senders: Arc<Mutex<HashSet<i64>>>,
    storage: Arc<Mutex<Box<dyn Storage>>>,
) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        let mut locked_senders = senders.lock().await;

        if locked_senders.contains(&msg.chat.id.0) {
            drop(locked_senders);

            bot.send_message(msg.chat.id, "Please wait...").await?;
            bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                .await?;
        } else {
            locked_senders.insert(msg.chat.id.0);

            drop(locked_senders);

            let bot_clone = bot.clone();
            let chat_id = msg.chat.id;
            let senders_clone = senders.clone();
            let text = text.to_string();
            let storage_clone = storage.clone();

            bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                .await?;

            tokio::spawn(async move {
                let answer = system::send_message(text, chat_id.0, storage_clone).await;
                if let Err(e) = bot_clone.send_message(chat_id, answer).await {
                    println!("Failed to send message: {}", e);
                }
                senders_clone.lock().await.remove(&chat_id.0);
            });
        }
    } else {
        invalid(bot, msg).await?
    }

    Ok(())
}
/// Invalid command handler
///
/// Responds to unrecognized bot commands
///
/// # Arguments
/// * `bot` - Telegram Bot instance
/// * `msg` - Message containing the invalid command
///
/// # Returns
/// * `ResponseResult<()>` - Result of sending the error message
pub async fn invalid(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, "Invalid command").await?;
    Ok(())
}
