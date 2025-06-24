use std::sync::Arc;

use async_trait::async_trait;
use tracing::{event, Level};

mod db_storage;
mod memory_storage;

use crate::{
    db,
    lm_types::Message,
    storage::{db_storage::DbStorage, memory_storage::MemoryStorage},
    CONFIG,
};

#[async_trait]
pub trait Storage: Send + Sync {
    async fn get_conversation_context(&self, chat_id: i64) -> Vec<Message>;
    async fn set_conversation_context(&self, chat_id: i64, context: Message);
    async fn clear_conversation_context(&self, chat_id: i64);

    async fn get_system_fingerprint(&self, chat_id: i64) -> String;
    async fn set_system_fingerprint(&self, chat_id: i64, fingerprint: String);

    async fn get_temperature(&self, chat_id: i64) -> f32;
    async fn set_temperature(&self, chat_id: i64, temperature: f32);
}

// Factory method for creating the required storage
pub async fn create_storage() -> Arc<dyn Storage> {
    if CONFIG.get_bool("enable_db").unwrap_or(false) {
        event!(Level::INFO, "Initializing database…");
        match db::sqlite::init_db().await {
            Ok(_) => {
                event!(Level::INFO, "Running bot with database enabled.");
                Arc::new(DbStorage::new().await)
            }
            Err(e) => {
                event!(
                    Level::ERROR,
                    "DB init error: {e}. Falling back to in‑memory."
                );
                Arc::new(MemoryStorage::new())
            }
        }
    } else {
        event!(Level::INFO, "Running bot with in‑memory storage.");
        Arc::new(MemoryStorage::new())
    }
}
