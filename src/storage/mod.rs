use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{Level, event};

mod db_storage;
mod memory_storage;

use crate::{
    CONFIG, db,
    lm_types::Message,
    storage::{db_storage::DbStorage, memory_storage::MemoryStorage},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub note_id: i64,
    pub chat_id: i64,
    pub user_id: u64,
    pub text: String,
}

impl ToString for Note {
    fn to_string(&self) -> String {
        format!(
            "Note id: `{}`, text: {}... \n",
            self.note_id,
            self.text.chars().take(30).collect::<String>()
        )
    }
}

/// Defines the interface for conversation storage implementations
///
/// This trait provides methods for managing conversation context, system fingerprints,
/// and temperature settings for individual chat sessions. Implementations must be
/// thread-safe (Send + Sync) and support asynchronous operations.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Retrieves conversation history for a chat
    ///
    /// # Arguments
    /// * `chat_id` - Unique identifier for the chat session
    ///
    /// # Returns
    /// Vector of messages representing the conversation history
    async fn get_conversation_context(&self, chat_id: i64) -> Vec<Message>;

    /// Adds a message to the conversation history
    ///
    /// # Arguments
    /// * `chat_id` - Unique identifier for the chat session
    /// * `context` - Message to add to the conversation history
    async fn set_conversation_context(&self, chat_id: i64, context: Message);

    /// Clears all conversation history for a chat
    ///
    /// # Arguments
    /// * `chat_id` - Unique identifier for the chat session
    async fn clear_conversation_context(&self, chat_id: i64);

    /// Retrieves the system fingerprint for a chat
    ///
    /// The system fingerprint defines the AI personality and behavior characteristics
    ///
    /// # Arguments
    /// * `chat_id` - Unique identifier for the chat session
    ///
    /// # Returns
    /// String containing the system fingerprint configuration
    async fn get_system_fingerprint(&self, chat_id: i64) -> String;

    /// Updates the system fingerprint for a chat
    ///
    /// # Arguments
    /// * `chat_id` - Unique identifier for the chat session
    /// * `fingerprint` - New system fingerprint configuration
    async fn set_system_fingerprint(&self, chat_id: i64, fingerprint: String);

    /// Retrieves the temperature setting for a chat
    ///
    /// Temperature controls the creativity/randomness of AI responses (0.0-2.0)
    ///
    /// # Arguments
    /// * `chat_id` - Unique identifier for the chat session
    ///
    /// # Returns
    /// Current temperature value as f32
    async fn get_temperature(&self, chat_id: i64) -> f32;

    /// Updates the temperature setting for a chat
    ///
    /// # Arguments
    /// * `chat_id` - Unique identifier for the chat session
    /// * `temperature` - New temperature value (0.0-2.0)
    async fn set_temperature(&self, chat_id: i64, temperature: f32);

    async fn add_note(&self, note: Note);
    async fn remove_note(&self, chat_id: i64, note_id: i64);
    async fn list_notes(&self, chat_id: i64) -> Vec<Note>;
    async fn erase_notes(&self, chat_id: i64);
}

/// Creates the appropriate storage implementation based on configuration
///
/// This factory function determines which storage backend to use based on the
/// `enable_db` configuration setting. It provides automatic fallback to in-memory
/// storage if database initialization fails.
///
/// # Returns
/// Thread-safe storage implementation wrapped in Arc
///
/// # Behavior
/// - If `enable_db` is true, attempts to initialize database storage
/// - If database initialization fails, falls back to in-memory storage
/// - If `enable_db` is false, uses in-memory storage
pub async fn create_storage() -> Arc<dyn Storage> {
    // Check if database storage is enabled in configuration
    let use_db = CONFIG.get_bool("enable_db").unwrap_or(false);

    if use_db {
        event!(Level::INFO, "Initializing database storage...");

        match db::sqlite::init_db().await {
            Ok(_) => {
                event!(Level::INFO, "Database initialized successfully");

                match DbStorage::new().await {
                    Ok(db_storage) => {
                        event!(Level::INFO, "Using database storage backend");
                        return Arc::new(db_storage);
                    }
                    Err(e) => {
                        event!(
                            Level::ERROR,
                            "Failed to create database storage: {}. Falling back to in-memory storage",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                event!(
                    Level::ERROR,
                    "Database initialization failed: {}. Falling back to in-memory storage",
                    e
                );
            }
        }
    } else {
        event!(Level::INFO, "Database storage disabled in configuration");
    }

    // Fallback to in-memory storage
    event!(Level::INFO, "Using in-memory storage backend");
    Arc::new(MemoryStorage::new())
}
