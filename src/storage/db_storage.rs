use sqlx::{Executor, Pool, Sqlite, query};
use std::sync::Arc;
use teloxide::types::ThreadId;
use tracing::{Level, event};

use async_trait::async_trait;

use crate::{
    CONFIG, Error, db,
    lm_types::Message,
    storage::{Note, Storage},
};

pub struct DbStorage {
    // Структура для работы с БД
    db: Arc<Pool<Sqlite>>,
    max_conv_len: usize,
}

impl DbStorage {
    pub async fn new() -> Result<Self, Error> {
        event!(Level::INFO, "Trying to init_db...");
        let db = db::sqlite::init_db().await;
        event!(Level::INFO, "init_db succeed!");
        if let Ok(db) = db {
            let db = Self {
                db: Arc::new(db),
                max_conv_len: CONFIG.get("max_conversation_len").unwrap_or(20),
            };
            event!(Level::INFO, "init_db return self!");
            return Ok(db);
        } else {
            panic!("Failed to initialize database: {:?}", db.err());
        }
    }
}

// Реализация трейта для DbStorage
#[async_trait]
impl Storage for DbStorage {
    // Реализация методов с использованием БД
    async fn get_conversation_context(&self, user_id: i64) -> Vec<Message> {
        let qr = query!("SELECT context_len FROM users WHERE user_id = $1", user_id)
            .fetch_one(&*self.db)
            .await;

        let max_conversation_len = self.max_conv_len as i64;
        if let Ok(row) = qr {
            if row.context_len > 0 {
                let len = if row.context_len > max_conversation_len {
                    max_conversation_len
                } else {
                    row.context_len
                };
                let qr = query!(
                    "SELECT message, responder FROM context WHERE user_id = $1 ORDER BY id DESC LIMIT $2",
                    user_id,
                    len
                ).fetch_all(&*self.db).await;
                if let Ok(rows) = qr {
                    let mut messages = Vec::new();
                    for row in rows {
                        messages.push(Message {
                            content: row.message,
                            role: row.responder,
                            reasoning: None,
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
        event!(
            Level::INFO,
            "Set conversation 1: {:?}",
            self.db
                .execute(query!(
                    "INSERT INTO context (user_id, message, responder) VALUES ($1, $2, $3)",
                    chat_id,
                    context.content,
                    context.role
                ))
                .await
        );
        event!(
            Level::INFO,
            "Update user context_len: {:?}",
            self.db
                .execute(query!(
                    "INSERT INTO users (user_id, context_len) 
                VALUES ($1, 1) 
            ON CONFLICT(user_id)
            DO UPDATE SET context_len = context_len + 1 WHERE user_id = $1",
                    chat_id
                ))
                .await
        );
    }

    async fn clear_conversation_context(&self, chat_id: i64) {
        event!(
            Level::INFO,
            "clear_conversation: {:?}",
            self.db
                .execute(query!(
                    "INSERT INTO users (user_id, context_len) 
                VALUES ($1, $2) 
            ON CONFLICT(user_id) 
                DO UPDATE SET context_len = 0 
                WHERE user_id = $1",
                    chat_id,
                    0
                ))
                .await
        );
    }

    async fn get_system_fingerprint(&self, chat_id: i64) -> String {
        let qr = query!("SELECT system FROM users WHERE user_id = $1", chat_id)
            .fetch_one(&*self.db)
            .await;
        if let Ok(row) = qr {
            return row.system.unwrap_or("".to_string());
        } else {
            return "".to_string();
        }
    }

    async fn set_system_fingerprint(&self, chat_id: i64, fingerprint: String) {
        event!(
            Level::INFO,
            "set_sestem_fingerprint: {:?}",
            self.db
                .execute(query!(
                    "INSERT INTO users(user_id, system, context_len) 
                VALUES ($1, $2, 0) 
            ON CONFLICT(user_id) 
                DO UPDATE SET system = $2 
                WHERE user_id = $1",
                    chat_id,
                    fingerprint
                ))
                .await
        );
    }

    async fn get_temperature(&self, chat_id: i64) -> f32 {
        let qr = query!("SELECT temperature FROM users WHERE user_id = $1", chat_id)
            .fetch_one(&*self.db)
            .await;
        if let Ok(row) = qr {
            return row.temperature.unwrap_or(0.7) as f32;
        } else {
            return 0.7;
        }
    }

    async fn set_temperature(&self, chat_id: i64, temperature: f32) {
        event!(
            Level::INFO,
            "Set_temperature: {:?}",
            self.db
                .execute(query!(
                    "INSERT INTO users(user_id, temperature, context_len) 
                VALUES ($1, $2, 0) 
            ON CONFLICT(user_id) 
                DO UPDATE SET temperature = $2 
                WHERE user_id = $1",
                    chat_id,
                    temperature
                ))
                .await
        );
    }

    async fn add_note(&self, note: Note) {
        todo!()
    }
    async fn remove_note(&self, chat_id: i64, note_id: i64) {
        todo!()
    }
    async fn list_notes(&self, chat_id: i64) -> Vec<Note> {
        todo!()
    }
    async fn erase_notes(&self, chat_id: i64) {
        todo!()
    }
    async fn enable(&self, chat_id: i64, thread_id: Option<i64>, is_super: bool) {
        todo!()
    }
    async fn disable(&self, chat_id: i64, thread_id: Option<i64>, is_super: bool) {
        todo!()
    }
    async fn is_enabled(&self, chat_id: i64, thread_id: Option<ThreadId>, is_super: bool) -> bool {
        todo!()
    }
}
