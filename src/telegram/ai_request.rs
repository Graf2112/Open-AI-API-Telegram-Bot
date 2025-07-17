use std::sync::Arc;

use teloxide::{
    prelude::Requester,
    types::{ChatAction, ChatId},
    Bot,
};
use tracing::{debug, warn, error};

use crate::{storage::Storage, system, telegram::message::BusySet};

/// Guard для автоматического удаления chat_id из BusySet
struct BusyGuard {
    busy: BusySet,
    chat_id: i64,
}

impl std::fmt::Debug for BusyGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BusyGuard({})", self.chat_id)
    }
}

// Добавьте логирование при создании/удалении
impl BusyGuard {
    fn new(busy: BusySet, chat_id: i64) -> Self {
        debug!("Creating guard for chat {}", chat_id);
        Self { busy, chat_id }
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        debug!("Removing chat {} from busy set", self.chat_id);
        self.busy.remove(&self.chat_id);
    }
}

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
    // Проверка и установка busy состояния
    if !busy.insert(chat_id.0) {
        if let Err(e) = bot.send_message(chat_id, "⏳ Please wait...").await {
            warn!("Failed to send wait message to {}: {:?}", chat_id, e);
        }
        return;
    }

    // Создаем guard сразу после успешной вставки
    let guard = BusyGuard::new(busy.clone(), chat_id.0);

    // Отправка typing indicator
    let typing_fut = bot.send_chat_action(chat_id, ChatAction::Typing);
    let ai_fut = system::reqwest_ai(text, chat_id.0, storage);
    
    let (typing_res, ai_res) = tokio::join!(typing_fut, ai_fut);

    if let Err(e) = typing_res {
        warn!("Failed to send typing action to {}: {:?}", chat_id, e);
    }

    // Обработка результата AI
    
    for chunk in ai_res {
        if let Err(e) = bot.send_message(chat_id, &chunk).await {
            error!("Failed to send message chunk to {}: {:?}", chat_id, e);
            break;
        }
    }
        

    // Явный дроп стража перед выходом
    drop(guard);
}
