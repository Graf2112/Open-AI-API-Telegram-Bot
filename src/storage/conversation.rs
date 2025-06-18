use std::collections::HashMap;

use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::{lm_types::Message, CONFIG};

//Store conversation history per user
static CONVERSATION: Lazy<Mutex<HashMap<i64, Vec<Message>>>> =
    Lazy::new(|| Mutex::new(HashMap::with_capacity(0)));

/// Clears conversation history for a specific user or all users
///
/// # Arguments
/// * `user_id` - User ID for which to clear conversation history
///
/// # Returns
/// * `()`
pub async fn clear_conversation_context(user_id: i64) {
    let mut conversations = CONVERSATION.lock().await;
    conversations.remove(&user_id);
}

/// Update or insert if not exist conversation history for a specific user
///
/// # Arguments
/// * `user_id` - User ID
/// * `message` - Message to be added to the conversation history
pub async fn update_or_insert_history(user_id: i64, message: Message) {
    let mut conversations = CONVERSATION.lock().await;
    let max_conversation_len = CONFIG.get("max_conversation_len").unwrap_or(20);
    let history = conversations.entry(user_id).or_insert_with(Vec::new);
    history.push(message);
    if history.len() > max_conversation_len {
        *history = history.split_off(history.len() - max_conversation_len);
    }
}

/// Get conversation history for a specific user
///
/// # Arguments
/// * `user_id` - User ID
///
/// # Returns
/// * `Vec<Message>` - Conversation history or empty vector if not found
pub async fn get_history(user_id: i64) -> Vec<Message> {
    let conversations = CONVERSATION.lock().await;
    conversations.get(&user_id).cloned().unwrap_or_default()
}
