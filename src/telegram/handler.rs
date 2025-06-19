//! Command Handler Module
//!
//! This module implements the telegram bot command handling functionality.
//! It processes user commands and manages interactions with the Llama AI model.
use crate::{storage::Storage, system};
use dashmap::DashSet;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{ChatAction, Message},
    utils::command::BotCommands,
    Bot,
};

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

pub async fn handle_ai_request(
    bot: Bot,
    chat_id: ChatId,
    text: String,
    storage: Arc<dyn Storage>,
    busy: BusySet,
) {
    if !busy.insert(chat_id.0) {
        let _ = bot.send_message(chat_id, "⏳ Please wait...").await;
        return;
    }

    let typing = bot.send_chat_action(chat_id, ChatAction::Typing);
    let req = system::reqwest_ai(text, chat_id.0, storage);

    let (_, result) = tokio::join!(typing, req);

    for chunk in result {
        let _ = bot.send_message(chat_id, chunk).await;
    }

    busy.remove(&chat_id.0);
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
    busy: BusySet,
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
            let chat_id = msg.chat.id;
            let bot_clone = bot.clone();
            let storage_clone = storage.clone();
            let busy_clone = busy.clone();

            tokio::spawn(async move {
                handle_ai_request(bot_clone, chat_id, text, storage_clone, busy_clone).await;
            });
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
    busy: BusySet,
    storage: Arc<dyn Storage>,
) -> ResponseResult<()> {
    if !msg.chat.is_group() {
        if let Some(text) = msg.text() {
            let chat_id = msg.chat.id;
            let bot_clone = bot.clone();
            let text = text.to_string();
            let storage_clone = storage.clone();
            let busy_clone = busy.clone();

            tokio::spawn(async move {
                handle_ai_request(bot_clone, chat_id, text, storage_clone, busy_clone).await;
            });
        } else {
            invalid(bot, msg).await?
        }
    }

    Ok(())
}

pub async fn inline_handler() -> ResponseResult<()> {
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
