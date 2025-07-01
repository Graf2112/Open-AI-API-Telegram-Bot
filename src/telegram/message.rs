//! Command Handler Module
//!
//! This module implements the telegram bot command handling functionality.
//! It processes user commands and manages interactions with the Llama AI model.
use crate::{storage::Storage, telegram::ai_request::handle_ai_request};
use dashmap::DashSet;
use std::sync::Arc;
use teloxide::{prelude::*, types::Message, Bot};

pub type BusySet = Arc<DashSet<i64>>;

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
    if !msg.chat.is_group() && !msg.chat.is_channel() && !msg.chat.is_supergroup() {
        if let Some(text) = msg.text() {
            let chat_id = msg.chat.id;
            let bot_clone = bot.clone();
            let text = text.to_string();
            let storage_clone = storage.clone();
            let busy_clone = busy.clone();

            tokio::spawn(async move {
                handle_ai_request(bot_clone, chat_id, text, storage_clone, busy_clone, false).await;
            });
        } else {
            invalid(bot, msg).await?
        }
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
