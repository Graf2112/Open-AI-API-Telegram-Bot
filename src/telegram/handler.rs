//! Command Handler Module
//!
//! This module implements the telegram bot command handling functionality.
//! It processes user commands and manages interactions with the Llama AI model.
use crate::{storage::Storage, system};
use std::{collections::HashSet, sync::Arc};
use dashmap::DashSet;
use teloxide::{prelude::*, types::Message, utils::command::BotCommands, Bot};
use tokio::sync::Mutex;

pub type BusySet = Arc<DashSet<i64>>;

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
    // Starts the bot and displays help message
    #[command(description = "bot conversation start. Use /help to get commands list.")]
    Start,
    // Displays list of available commands
    #[command(description = "displays description of all commands.")]
    Help,
    // Processes chat requests with Llama AI
    // Takes a String parameter containing the user's prompt
    #[command(description = "place your promt after this command. It will be sent to the model.")]
    Chat(String),
    // Clears conversation history
    #[command(description = "clears conversation context.")]
    Clear,
    // Sets system fingerprint for the model
    #[command(description = "set system fingerprint..")]
    System(String),
    // Sets temperature for the model
    #[command(description = "set temperature for model. Choose from 0.0 to 1.0. Default is 0.7.")]
    Temperature(f32),
    // Stops current operation
    #[command(description = "stops current operation.")]
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
    storage: Arc<dyn Storage>,
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
                    let answer = system::reqwest_ai(text, chat_id.0, storage_clone).await;

                    for ans in answer {
                        if let Err(e) = bot_clone
                            // .parse_mode(teloxide::types::ParseMode::Markdown)
                            .send_message(chat_id, ans)
                            .await
                        {
                            println!("Failed to send message: {}", e);
                        }
                    }
                    senders_clone.lock().await.remove(&chat_id.0);
                });
            }
        }
        Command::System(fingerprint) => {
            storage
                .set_system_fingerprint(msg.chat.id.0, fingerprint)
                .await;
            bot.send_message(msg.chat.id, "System fingerprint set")
                .await?;
        }
        Command::Temperature(temperature) => {
            let mut temperature = temperature as f32;

            if !{ 0.0..=1.0 }.contains(&temperature) {
                temperature = 0.7;
            }

            storage.set_temperature(msg.chat.id.0, temperature).await;
            bot.send_message(msg.chat.id, "Temperature set").await?;
        }
        Command::Clear => {
            storage.clear_conversation_context(msg.chat.id.0).await;
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
    storage: Arc<dyn Storage>,
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
                let answer = system::reqwest_ai(text, chat_id.0, storage_clone).await;
                for ans in answer {
                    if let Err(e) = bot_clone
                        // .parse_mode(teloxide::types::ParseMode::Markdown)
                        .send_message(chat_id, ans)
                        .await
                    {
                        println!("Failed to send message: {}", e);
                    }
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
