use dashmap::DashMap;

use async_trait::async_trait;

use crate::{lm_types::Message, storage::Storage, CONFIG};

// Реализации хранилищ
pub struct MemoryStorage {
    context: DashMap<i64, Vec<Message>>,
    fingerprint: DashMap<i64, String>,
    temperature: DashMap<i64, f32>,
    max_conv_len: usize,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            // Инициализация структуры
            context: DashMap::with_capacity(100),
            fingerprint: DashMap::with_capacity(100),
            temperature: DashMap::with_capacity(100),
            max_conv_len: CONFIG.get("max_conversation_len").unwrap_or(20),
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
