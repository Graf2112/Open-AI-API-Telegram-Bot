use dashmap::DashMap;
use sqlx::{query, Executor, Pool, Sqlite};
use std::sync::Arc;

use async_trait::async_trait;

use crate::{db, lm_types::Message, CONFIG};

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

// Фабричный метод для создания нужного хранилища
pub async fn create_storage() -> Arc<dyn Storage> {
    if CONFIG.get_bool("enable_db").unwrap_or(false) {
        println!("Initializing database…");
        match db::sqlite::init_db().await {
            Ok(_) => {
                println!("Running bot with database enabled.");
                Arc::new(DbStorage::new().await)
            }
            Err(e) => {
                eprintln!("DB init error: {e}. Falling back to in‑memory.");
                Arc::new(MemoryStorage::new())
            }
        }
    } else {
        println!("Running bot with in‑memory storage.");
        Arc::new(MemoryStorage::new())
    }
}

// Реализации хранилищ
struct MemoryStorage {
    context: DashMap<i64, Vec<Message>>,
    fingerprint: DashMap<i64, String>,
    temperature: DashMap<i64, f32>,
    max_conv_len: usize,
}

struct DbStorage {
    // Структура для работы с БД
    db: Arc<Pool<Sqlite>>,
}

impl MemoryStorage {
    fn new() -> Self {
        Self {
            // Инициализация структуры
            context: DashMap::with_capacity(100),
            fingerprint: DashMap::with_capacity(100),
            temperature: DashMap::with_capacity(100),
            max_conv_len: CONFIG.get("max_conversation_len").unwrap_or(20),
        }
    }
}

impl DbStorage {
    async fn new() -> Self {
        let db = db::sqlite::init_db().await;
        if let Ok(db) = db {
            Self { db: Arc::new(db) }
        } else {
            panic!("Failed to initialize database: {:?}", db.err());
        }
    }
}

// Реализация трейта для MemoryStorage
#[async_trait]
impl Storage for MemoryStorage {
    // Реализация методов с использованием текущей логики хранения в памяти
    async fn get_conversation_context(&self, user_id: i64) -> Vec<Message> {
        self.context
            .get(&user_id)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }

    async fn set_conversation_context(&self, user_id: i64, context: Message) {
        self.context
            .entry(user_id)
            .and_modify(|history| {
                history.push(context.clone());
                if history.len() > self.max_conv_len {
                    history.drain(..history.len() - self.max_conv_len);
                }
            })
            .or_insert_with(|| vec![context]);
    }

    async fn clear_conversation_context(&self, user_id: i64) {
        self.context.remove(&user_id);
    }

    async fn get_system_fingerprint(&self, user_id: i64) -> String {
        self.fingerprint
            .get(&user_id)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    async fn set_system_fingerprint(&self, user_id: i64, fingerprint: String) {
        self.fingerprint.insert(user_id, fingerprint);
    }

    async fn get_temperature(&self, user_id: i64) -> f32 {
        self.temperature.get(&user_id).map(|v| *v).unwrap_or(0.7)
    }

    async fn set_temperature(&self, user_id: i64, temperature: f32) {
        self.temperature.insert(user_id, temperature);
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

        let max_conversation_len = CONFIG.get("max_conversation_len").unwrap_or(20);
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
        println!(
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
        println!(
            "Update user context_len: {:?}",
            self.db
                .execute(query!(
                    "INSERT INTO users (user_id, context_len) 
                VALUES ($1, 0) 
            ON CONFLICT(user_id)
            DO UPDATE SET context_len = context_len + 1 WHERE user_id = $1",
                    chat_id
                ))
                .await
        );
    }

    async fn clear_conversation_context(&self, chat_id: i64) {
        println!(
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
        println!(
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
        println!(
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
}
