pub mod conversation;
pub mod fingerprint;
pub mod temperature;

use std::sync::Arc;
use sqlx::{query, Pool, Sqlite};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{db, lm_types::Message};

#[async_trait]
pub trait Storage {
    async fn get_conversation_context(&self, chat_id: i64) -> Vec<Message>;
    async fn set_conversation_context(&self, chat_id: i64, context: Message);
    async fn clear_conversation_context(&self, chat_id: i64);
    
    async fn get_system_fingerprint(&self, chat_id: i64) -> String;
    async fn set_system_fingerprint(&self, chat_id: i64, fingerprint: String);
    
    async fn get_temperature(&self, chat_id: i64) -> f32;
    async fn set_temperature(&self, chat_id: i64, temperature: f32);
}

// Фабричный метод для создания нужного хранилища
pub async fn create_storage(db_enabled: bool) -> Box<dyn Storage + Send + Sync> {
    if db_enabled {
        Box::new(DbStorage::new().await)
    } else {
        Box::new(MemoryStorage::new())
    }
}

// Реализации хранилищ
struct MemoryStorage {
    // Ваши текущие структуры для хранения в памяти
}

struct DbStorage {
    // Структура для работы с БД
    db: Arc<Mutex<Pool<Sqlite>>>
}

impl MemoryStorage {
    fn new() -> Self {
        Self {
            // Инициализация структуры
        }
    }
}

impl DbStorage {
    async fn new() -> Self {
        let db = db::sqlite::init_db().await;
        if let Err(e) = db {
            panic!("Failed to initialize database: {}", e);
        } else {
            Self {
                db: Arc::new(Mutex::new(db.unwrap()))
            }
        }
    }
}

// Реализация трейта для MemoryStorage
#[async_trait]
impl Storage for MemoryStorage {
    // Реализация методов с использованием текущей логики хранения в памяти
    async fn get_conversation_context(&self, user_id: i64) -> Vec<Message> {
        conversation::get_history(user_id).await
    }

    async fn set_conversation_context(&self, user_id: i64, context: Message) {
        conversation::update_or_insert_history(user_id, context).await
    }

    async fn clear_conversation_context(&self, user_id: i64) {
        conversation::clear_conversation_context(user_id).await
    }
    
    async fn get_system_fingerprint(&self, user_id: i64) -> String {
        fingerprint::get_system_fingerprint(user_id).await
    }

    async fn set_system_fingerprint(&self, user_id: i64, fingerprint: String) {
        fingerprint::set_system_fingerprint(user_id, fingerprint).await
    }
    
    async fn get_temperature(&self, user_id: i64) -> f32 {
        temperature::get_temperature(user_id).await
    }

    async fn set_temperature(&self, user_id: i64, temperature: f32) {
        temperature::set_temperature(user_id, temperature).await
    }
}

// Реализация трейта для DbStorage
#[async_trait]
impl Storage for DbStorage {
    // Реализация методов с использованием БД
    async fn get_conversation_context(&self, user_id: i64) -> Vec<Message> {
        let db = self.db.lock().await;
        let qr = query!("SELECT context_len FROM users WHERE user_id = $1", user_id).fetch_one(&*db).await;
        if let Ok(row) = qr {
            if row.context_len > 0 {
                let len = if row.context_len > 20 {20} else {row.context_len};
                let qr = query!(
                    "SELECT message, responder FROM context WHERE user_id = $1 ORDER BY id DESC LIMIT $2",
                    user_id,
                    len
                ).fetch_all(&*db).await;
                if let Ok(rows) = qr {
                    let mut messages = Vec::new();
                    for row in rows {
                        messages.push(Message {
                            content: row.message,
                            role: row.responder.expect("User"),
                        });
                    }
                    messages.reverse();
                    return messages;
                }
            }
        }
        vec![]
    }

    async fn set_conversation_context(&self, chat_id: i64, context: Message) {

    }

    async fn clear_conversation_context(&self, chat_id: i64) {

    }
    
    async fn get_system_fingerprint(&self, chat_id: i64) -> String {
        "".to_string()
    }
    
    async fn set_system_fingerprint(&self, chat_id: i64, fingerprint: String) {

    }
    
    async fn get_temperature(&self, chat_id: i64) -> f32 {
        0.7
    }

    async fn set_temperature(&self, chat_id: i64, temperature: f32) {

    } 
}