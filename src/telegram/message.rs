//! Command Handler Module
//!
//! This module implements the telegram bot command handling functionality.
//! It processes user commands and manages interactions with the Llama AI model.
use crate::{storage::Storage, telegram::ai_request::handle_ai_request};
use dashmap::DashSet;
use log::info;
use std::sync::Arc;
use teloxide::{
    prelude::*, types::{ChatKind, False, Message}, Bot
};
use tracing::warn;

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
    bot_id: UserId,
) -> ResponseResult<()> {
    if let Some(user) = &msg.from {
        let chat_id = msg.chat.id;
        let thread_id = msg.thread_id;
        
        
        // Обработка разных типов чатов
        let enabled = match msg.chat.clone().kind {
            ChatKind::Private(_) => {},
            ChatKind::Public(chat_public) => match chat_public.kind {
                teloxide::types::PublicChatKind::Channel(public_chat_channel) => {return Ok(());},
                teloxide::types::PublicChatKind::Group => {},
                teloxide::types::PublicChatKind::Supergroup(public_chat_supergroup) => match public_chat_supergroup.is_forum {
                    true => {},
                    false => {},
                },
            },
        };


        if !msg.chat.is_private() {
            if !msg
                .reply_to_message()
                .is_some_and(|reply| reply.from.as_ref().is_some_and(|u| u.id == bot_id))
            {
                return Ok(());
            }
        }

        let Some(text) = &msg.text() else {
            return Ok(());
        };

        let message_id = msg.id;
        let text = format!(
            "{{Username: {} (@{}), DateTime: {}, Message: {}}}",
            user.full_name(),
            user.username.clone().unwrap_or("".to_owned()),
            chrono::Local::now(),
            text
        );

        // Clone necessary resources for async task
        let bot_clone = bot.clone();
        let storage_clone = storage.clone();
        let busy_clone = busy.clone();

        if !msg.chat.is_private() {
            handle_ai_request(
                bot_clone,
                chat_id,
                message_id,
                text,
                storage_clone,
                busy_clone,
            )
            .await;
        } else {
            tokio::spawn(async move {
                handle_ai_request(
                    bot_clone,
                    chat_id,
                    message_id,
                    text,
                    storage_clone,
                    busy_clone,
                )
                .await;
            });
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
    warn!("Invalid command received from chat {}", msg.chat.id);
    bot.send_message(
        msg.chat.id,
        "❌ Invalid command. Use /help to see available commands.",
    )
    .await?;
    Ok(())
}
