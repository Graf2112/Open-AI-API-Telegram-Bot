use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use teloxide::types::ThreadId;
use tracing::{Level, event};

mod db_storage;
mod memory_storage;

use crate::{
    CONFIG, db,
    lm_types::Message,
    storage::{db_storage::DbStorage, memory_storage::MemoryStorage},
};

/// Represents a user note stored in the system
///
/// Notes are text snippets associated with specific users in specific chats.
/// Each note has a unique identifier within its chat context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Unique identifier of the note within a chat
    pub note_id: i64,

    /// Chat identifier where the note is stored
    pub chat_id: i64,

    /// User identifier who created the note
    pub user_id: u64,

    /// Content of the note
    pub text: String,
}

impl ToString for Note {
    fn to_string(&self) -> String {
        let preview = self.text.chars().take(30).collect::<String>();

        format!(
            "Note #{}: {}...\n",
            self.note_id,
            if self.text.len() > 30 {
                format!("{}", preview)
            } else {
                preview
            }
        )
    }
}

/// Represents chat-specific configuration settings
///
/// Controls bot functionality at both chat and thread levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSettings {
    /// Indicates if the chat is a supergroup
    pub is_supergroup: bool,

    /// Thread-specific enablement status
    ///
    /// Maps thread IDs to their activation status:
    /// - `true`: bot enabled in thread
    /// - `false`: bot disabled in thread
    pub threads: HashMap<i64, bool>,

    /// Global bot enablement status for the chat
    pub enabled: bool,
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

    // --- Note Management ---

    /// Adds a new note to storage
    ///
    /// # Arguments
    /// * `note` - Complete note object to store
    ///
    /// # Implementation Notes
    /// - Should generate unique note_id if not set
    /// - Should validate note ownership
    async fn add_note(&self, note: Note);

    /// Removes a specific note
    ///
    /// # Arguments
    /// * `chat_id` - Chat where the note exists
    /// * `note_id` - Identifier of the note to remove
    ///
    /// # Errors
    /// Implementations should silently handle missing notes
    async fn remove_note(&self, chat_id: i64, note_id: i64);
    /// Lists all notes in a chat
    ///
    /// # Arguments
    /// * `chat_id` - Chat to retrieve notes from
    ///
    /// # Returns
    /// Vector of notes sorted by creation time (newest first)
    async fn list_notes(&self, chat_id: i64) -> Vec<Note>;

    /// Deletes all notes in a chat
    async fn erase_notes(&self, chat_id: i64);
    // --- Chat Configuration ---

    /// Enables bot functionality in a chat/thread
    ///
    /// # Arguments
    /// * `chat_id` - Target chat
    /// * `thread_id` - Optional thread identifier:
    ///     - `None`: Enable globally for chat
    ///     - `Some(id)`: Enable for specific thread
    async fn enable(&self, chat_id: i64, thread_id: Option<i64>, is_super: bool);

    /// Disables bot functionality in a chat/thread
    ///
    /// See `enable()` for parameter details
    async fn disable(&self, chat_id: i64, thread_id: Option<i64>, is_super: bool);

    /// Checks if bot is enabled in a chat/thread
    ///
    /// # Arguments
    /// * `chat_id` - Target chat
    /// * `thread_id` - Optional thread identifier
    ///
    /// # Returns
    /// `true` if bot is enabled in the specified context
    ///
    /// # Evaluation Order
    /// 1. If thread_id provided, check thread-specific setting
    /// 2. If not enabled in thread, check global chat setting
    /// 3. Returns false if both not enabled
    async fn is_enabled(&self, chat_id: i64, thread_id: Option<ThreadId>, is_super: bool) -> bool;
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
/// 1. Checks `enable_db` configuration:
///    - If false: uses in-memory storage
///    - If true: attempts database initialization
/// 2. Database initialization:
///    - On success: creates database-backed storage
///    - On failure: falls back to in-memory storage
/// 3. Returns storage instance with timing metrics
pub async fn create_storage() -> Arc<dyn Storage> {
    let start_time = std::time::Instant::now();

    // Determine storage type from configuration
    let storage_type = match CONFIG.get_bool("enable_db") {
        Ok(true) => "database",
        Ok(false) => "memory",
        Err(e) => {
            event!(
                Level::ERROR,
                "Invalid enable_db config: {}. Using memory storage",
                e
            );
            "memory"
        }
    };

    // Early return for memory storage
    if storage_type == "memory" {
        event!(Level::INFO, "Using in-memory storage backend");
        return create_memory_storage(start_time).await;
    }

    // Attempt database storage initialization
    match try_create_db_storage().await {
        Ok(storage) => {
            event!(
                Level::INFO,
                "Database storage initialized in {:?}",
                start_time.elapsed()
            );
            storage
        }
        Err(e) => {
            event!(
                Level::ERROR,
                "Database storage failed: {}. Falling back to memory",
                e
            );
            create_memory_storage(start_time).await
        }
    }
}

/// Attempts to create a database-backed storage instance
async fn try_create_db_storage() -> Result<Arc<dyn Storage>, String> {
    event!(Level::INFO, "Initializing database storage...");

    db::sqlite::init_db()
        .await
        .map_err(|e| format!("Database initialization failed: {}", e))?;

    DbStorage::new()
        .await
        .map(|storage| {
            event!(Level::INFO, "Database storage created successfully");
            Arc::new(storage) as Arc<dyn Storage>
        })
        .map_err(|e| format!("Failed to create DB storage: {}", e))
}

/// Creates an in-memory storage instance
async fn create_memory_storage(start_time: std::time::Instant) -> Arc<dyn Storage> {
    let storage = Arc::new(MemoryStorage::new());
    event!(
        Level::INFO,
        "In-memory storage initialized in {:?}",
        start_time.elapsed()
    );
    storage
}
