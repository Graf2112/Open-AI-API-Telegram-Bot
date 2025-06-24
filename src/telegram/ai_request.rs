use std::sync::Arc;

use teloxide::{
    prelude::Requester,
    types::{ChatAction, ChatId},
    Bot,
};

use crate::{storage::Storage, system, telegram::message::BusySet};

/// Handles an AI request for a specific chat
///
/// Manages the AI interaction process by:
/// - Preventing concurrent requests for the same chat
/// - Sending a typing indicator
/// - Requesting AI response
/// - Sending response chunks
/// - Clearing the busy state after completion
///
/// # Arguments
/// * `bot` - Telegram Bot instance for sending messages
/// * `chat_id` - Unique identifier for the chat
/// * `text` - User's input text
/// * `storage` - Storage interface for AI request context
/// * `busy` - Thread-safe set tracking active chat requests
///
/// # Remarks
/// This function runs asynchronously and handles the entire AI request lifecycle
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
