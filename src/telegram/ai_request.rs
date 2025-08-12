//! AI Request Handler Module
//!
//! This module handles AI requests from Telegram users, managing the complete
//! lifecycle from request to response delivery.

use std::sync::Arc;
use teloxide::{
    prelude::Requester,
    types::{ChatAction, ChatId},
    Bot, RequestError,
};
use tracing::{error, info, warn, debug};

use crate::{storage::Storage, system, telegram::message::BusySet};

/// Result type for AI request handling operations
pub type AiRequestResult<T> = Result<T, AiRequestError>;

/// Errors that can occur during AI request handling
#[derive(Debug, thiserror::Error)]
pub enum AiRequestError {
    #[error("Telegram API error: {0}")]
    TelegramError(#[from] RequestError),
    #[error("AI processing error: {0}")]
    AiProcessingError(String),
    #[error("Chat is busy processing another request")]
    ChatBusy,
}

/// Handles an AI request for a specific chat with comprehensive error handling
///
/// This function manages the complete AI interaction lifecycle:
/// - Prevents concurrent requests for the same chat
/// - Shows typing indicator to the user
/// - Processes the AI request
/// - Sends response chunks to the user
/// - Ensures cleanup of busy state
///
/// # Arguments
/// * `bot` - Telegram Bot instance for sending messages
/// * `chat_id` - Unique identifier for the target chat
/// * `text` - User's input text to process
/// * `storage` - Storage interface for maintaining conversation context
/// * `busy` - Thread-safe set tracking currently active chat requests
/// * `is_assistant_mode` - Whether to use assistant mode for responses
///
/// # Returns
/// * `AiRequestResult<()>` - Success or detailed error information
///
/// # Examples
/// ```rust
/// let result = handle_ai_request(
///     bot,
///     chat_id,
///     "Hello AI!".to_string(),
///     storage,
///     busy_set,
///     false
/// ).await;
/// ```
pub async fn handle_ai_request(
    bot: Bot,
    chat_id: ChatId,
    text: String,
    storage: Arc<dyn Storage>,
    busy: BusySet,
    is_assistant_mode: bool,
) -> AiRequestResult<()> {
    debug!("Processing AI request for chat {}: {}", chat_id, text);

    // Ensure this chat isn't already processing a request
    if !busy.insert(chat_id.0) {
        warn!("Chat {} is already busy, rejecting new request", chat_id);
        send_busy_message(&bot, chat_id).await?;
        return Err(AiRequestError::ChatBusy);
    }

    // Use RAII pattern to ensure cleanup on any exit path
    let _guard = BusyGuard::new(busy.clone(), chat_id.0);

    info!("Starting AI request processing for chat {}", chat_id);

    // Start typing indicator and AI processing concurrently
    let typing_task = send_typing_indicator(&bot, chat_id);
    let ai_task = process_ai_request(text, chat_id.0, storage, is_assistant_mode);

    let (typing_result, ai_result) = tokio::join!(typing_task, ai_task);

    // Log typing indicator result (non-critical)
    if let Err(e) = typing_result {
        warn!("Failed to send typing indicator for chat {}: {}", chat_id, e);
    }

    // Handle AI processing result
    let response_chunks = ai_result.map_err(|e| {
        error!("AI processing failed for chat {}: {}", chat_id, e);
        AiRequestError::AiProcessingError(e)
    })?;

    // Send response chunks to user
    send_response_chunks(&bot, chat_id, response_chunks).await?;

    info!("Successfully completed AI request for chat {}", chat_id);
    Ok(())
}

/// Sends a busy message to inform the user about ongoing processing
async fn send_busy_message(bot: &Bot, chat_id: ChatId) -> Result<(), RequestError> {
    bot.send_message(chat_id, "⏳ Please wait, I'm still processing your previous request...")
        .await?;
    Ok(())
}

/// Sends typing indicator to show the bot is processing
async fn send_typing_indicator(bot: &Bot, chat_id: ChatId) -> Result<(), RequestError> {
    bot.send_chat_action(chat_id, ChatAction::Typing).await?;
    Ok(())
}

/// Processes the AI request and returns response chunks
async fn process_ai_request(
    text: String,
    chat_id: i64,
    storage: Arc<dyn Storage>,
    _is_assistant_mode: bool, // Parameter kept for future use
) -> Result<Vec<String>, String> {
    debug!("Making AI request for chat {}", chat_id);
    
    // Call the system AI function - returns Vec<String> directly
    let chunks = system::reqwest_ai(text, chat_id, storage).await;
    
    if chunks.is_empty() {
        Err("AI returned empty response".to_string())
    } else {
        Ok(chunks)
    }
}

/// Sends response chunks to the user with error handling
async fn send_response_chunks(
    bot: &Bot,
    chat_id: ChatId,
    chunks: Vec<String>,
) -> AiRequestResult<()> {
    if chunks.is_empty() {
        warn!("No response chunks to send for chat {}", chat_id);
        bot.send_message(chat_id, "❌ Sorry, I couldn't generate a response. Please try again.")
            .await?;
        return Ok(());
    }

    for (index, chunk) in chunks.iter().enumerate() {
        debug!("Sending chunk {} of {} to chat {}", index + 1, chunks.len(), chat_id);
        
        if let Err(e) = bot.send_message(chat_id, chunk).await {
            error!("Failed to send chunk {} to chat {}: {}", index + 1, chat_id, e);
            
            // Try to send an error message
            let _ = bot.send_message(
                chat_id,
                "❌ Sorry, there was an error sending the response."
            ).await;
            
            return Err(AiRequestError::TelegramError(e));
        }
    }

    debug!("Successfully sent {} chunks to chat {}", chunks.len(), chat_id);
    Ok(())
}

/// RAII guard to ensure busy state is cleaned up
struct BusyGuard {
    busy: BusySet,
    chat_id: i64,
}

impl BusyGuard {
    fn new(busy: BusySet, chat_id: i64) -> Self {
        Self { busy, chat_id }
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        debug!("Cleaning up busy state for chat {}", self.chat_id);
        self.busy.remove(&self.chat_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_busy_guard_cleanup() {
        let busy = Arc::new(dashmap::DashSet::new());
        let chat_id = 12345i64;
        
        // Insert and create guard
        busy.insert(chat_id);
        {
            let _guard = BusyGuard::new(busy.clone(), chat_id);
            assert!(busy.contains(&chat_id));
        } // Guard drops here
        
        // Should be cleaned up
        assert!(!busy.contains(&chat_id));
    }

    #[test]
    fn test_ai_request_error_display() {
        let error = AiRequestError::ChatBusy;
        assert_eq!(error.to_string(), "Chat is busy processing another request");
        
        let error = AiRequestError::AiProcessingError("Test error".to_string());
        assert_eq!(error.to_string(), "AI processing error: Test error");
    }
}
