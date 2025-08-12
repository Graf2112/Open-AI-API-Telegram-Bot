use std::collections::HashMap;

use dashmap::DashMap;

use async_trait::async_trait;
use teloxide::types::ThreadId;
use tracing::info;

use crate::{
    CONFIG,
    lm_types::Message,
    storage::{ChatSettings, Note, Storage},
};

/// In-memory storage implementation using DashMap for thread safety
///
/// This storage backend keeps all data in memory using concurrent hash maps.
/// Suitable for development, testing, and small-scale deployments.
///
/// # Data Structures
/// - `context`: Conversation history per chat
/// - `fingerprint`: AI personality settings per chat
/// - `temperature`: Creativity settings per chat
/// - `notes`: User notes organized by chat
/// - `chats`: Chat configuration settings
pub struct MemoryStorage {
    context: DashMap<i64, Vec<Message>>,
    fingerprint: DashMap<i64, String>,
    temperature: DashMap<i64, f32>,
    notes: DashMap<i64, Vec<Note>>, // chat_id -> (note_id -> Note)
    chats: DashMap<i64, ChatSettings>,
    max_conv_len: usize,
}

impl MemoryStorage {
    /// Creates a new in-memory storage instance
    ///
    /// Initializes all storage maps with default capacities and loads
    /// configuration values like `max_conversation_len`.
    pub fn new() -> Self {
        Self {
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
                drop(notes);
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

    async fn enable(&self, chat_id: i64, thread_id: Option<i64>, is_super: bool) {
        info!("enable: {:?} {:?}", chat_id, thread_id);
        self.chats
            .entry(chat_id)
            .and_modify(|settings| {
                if let Some(tid) = thread_id {
                    info!("Enable {}", tid);
                    settings.threads.insert(tid, true);
                } else {
                    info!("Enable");
                    settings.enabled = true;
                }
            })
            .or_insert_with(|| ChatSettings {
                is_supergroup: is_super,
                threads: if let Some(tid) = thread_id {
                    HashMap::from([(tid, false)])
                } else {
                    HashMap::new()
                },
                enabled: true,
            });

        info!("enable2: {:?}", self.chats);
    }
    async fn disable(&self, chat_id: i64, thread_id: Option<i64>, is_super: bool) {
        info!("disable: {:?} {:?}", chat_id, thread_id);
        self.chats
            .entry(chat_id)
            .and_modify(|settings| {
                if let Some(tid) = thread_id {
                    info!("Diable {}", tid);
                    settings.threads.insert(tid, false);
                } else {
                    info!("Diable chat");
                    settings.enabled = false;
                }
            })
            .or_insert_with(|| ChatSettings {
                is_supergroup: is_super,
                threads: if let Some(tid) = thread_id {
                    HashMap::from([(tid, false)])
                } else {
                    HashMap::new()
                },
                enabled: false,
            });

        info!("disable2: {:?}", self.chats);
    }
    async fn is_enabled(&self, chat_id: i64, thread_id: Option<ThreadId>, is_super: bool) -> bool {
        let chat = self.chats.get(&chat_id).map(|entry| entry.clone());

        info!("Chat: {:?}", chat);
        if let Some(chat) = chat {
            if !chat.is_supergroup || thread_id.is_none() {
                info!("Enabled: {}", chat.enabled);
                return chat.enabled;
            }
            if let Some(thread_id) = thread_id {
                info!("Thread id: {:?}", thread_id);
                let tid = thread_id.0.0 as i64;
                let chat_thread = chat.threads.get(&tid).unwrap_or(&true);
                info!("Thread info: {:?}", chat_thread);
                return *chat_thread;
            } else {
                return chat.enabled;
            }
        } else {
            return true;
        }
    }
}
