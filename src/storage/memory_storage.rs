use dashmap::DashMap;

use async_trait::async_trait;

use crate::{
    CONFIG,
    lm_types::Message,
    storage::{ChatSettings, Note, Storage},
};

// Реализации хранилищ
pub struct MemoryStorage {
    context: DashMap<i64, Vec<Message>>,
    fingerprint: DashMap<i64, String>,
    temperature: DashMap<i64, f32>,
    notes: DashMap<i64, Vec<Note>>,
    chats: DashMap<i64, ChatSettings>,
    max_conv_len: usize,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            // Инициализация структуры
            context: DashMap::with_capacity(100),
            fingerprint: DashMap::with_capacity(100),
            temperature: DashMap::with_capacity(100),
            notes: DashMap::with_capacity(100),
            chats: DashMap::with_capacity(100),
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

    async fn add_note(&self, note: Note) {
        self.notes
            .entry(note.chat_id)
            .and_modify(|notes| {
                notes.push(note.clone());
            })
            .or_insert_with(|| vec![note]);
    }

    async fn remove_note(&self, chat_id: i64, note_id: i64) {
        if let Some(mut notes) = self.notes.get_mut(&chat_id) {
            notes.retain(|note| note.note_id != note_id);
            if notes.is_empty() {
                drop(notes); // Явно отпускаем мутабельную ссылку
                self.notes.remove(&note_id);
            }
        }
    }

    async fn list_notes(&self, chat_id: i64) -> Vec<Note> {
        self.notes
            .get(&chat_id)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }
    async fn erase_notes(&self, chat_id: i64) {
        self.notes.remove(&chat_id);
    }

    async fn enable(&self, chat_id: i64, thread_id: Option<i64>) {
        todo!()
    }
    async fn disable(&self, chat_id: i64, thread_id: Option<i64>) {
        todo!()
    }
    async fn is_enabled(&self, chat_id: i64, thread_id: Option<i64>) -> bool {
        todo!()
    }
}
