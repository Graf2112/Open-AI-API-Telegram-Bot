//! Command Handler Module
//!
//! This module implements the telegram bot command handling functionality.
//! It processes user commands and manages interactions with the Llama AI model.
use crate::{storage::Storage, telegram::ai_request::handle_ai_request};
use dashmap::DashSet;
use tracing::warn;
use std::sync::Arc;
use teloxide::{prelude::*, types::Message, Bot};

pub type BusySet = Arc<DashSet<i64>>;

/// Message handler
/// Alternative of /chat command
///
/// # Arguments
/// * `bot` - Telegram Bot instance
/// * `msg` - Incoming message containing the command
/// * `busy` - Thread-safe set of chat IDs with active processing
/// * `storage` - Storage implementation for context management
///
/// # Returns
/// * `ResponseResult<()>` - Result of the command execution
pub async fn message_handler(
    bot: Bot,
    msg: Message,
    busy: BusySet,
    storage: Arc<dyn Storage>,
) -> ResponseResult<()> {
    // Only process private chats
    if !msg.chat.is_private() {
        return Ok(());
    }

    let Some(text) = msg.text() else {
        return invalid(bot, msg).await;
    };

    let chat_id = msg.chat.id;
    let text = text.to_string();

    // Clone necessary resources for async task
    let bot_clone = bot.clone();
    let storage_clone = storage.clone();
    let busy_clone = busy.clone();

    tokio::spawn(async move {
        handle_ai_request(
            bot_clone,
            chat_id,
            text,
            storage_clone,
            busy_clone,
        )
        .await;
    });

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
    warn!("Invalid command received from chat {}", msg.chat.id);
    bot.send_message(
        msg.chat.id,
        "❌ Invalid command. Use /help to see available commands.",
    )
    .await?;
    Ok(())
}
