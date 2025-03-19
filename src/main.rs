//! Llama AI Telegram Bot
//!
//! Main application entry point that initializes and runs the Telegram bot
//! with Llama AI integration. Handles configuration loading and dispatcher setup.

use config::Config;
use lazy_static::lazy_static;
use std::{collections::HashSet, sync::Arc};
use telegram::get_storage_handler;
use teloxide::prelude::*;
use tokio::sync::Mutex;

mod db;
mod lm_types;
mod storage;
mod system;
mod telegram;

lazy_static! {
    /// Global configuration instance
    /// Initialized once and available throughout the application
    static ref CONFIG: Config = system::get_config().expect("Unable to init config.");
}

/// Custom error type for the application
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Application entry point
///
/// Initializes the bot with configuration, sets up command handlers,
/// and starts the message dispatcher.
///
/// # Returns
/// * `Result<(), Error>` - Success or error status of bot execution
#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut db_enabled = false;

    println!("Preconfigure...");
    // Initialize database if enabled in configuration
    if CONFIG.get_bool("enable_db").unwrap_or(false) {
        println!("Initializing database...");
        // Initialize database
        if db::sqlite::init_db().await.is_err() {
            println!("Failed to initialize database: ");
            return Ok(());
        };
        println!("Running bot with database enabled.");
        db_enabled = true;
    } else {
        // Configure message handler tree
        println!("Running bot with in-memory storage.");
    }

    // Load bot token from configuration
    let token = CONFIG.get_string("token").unwrap_or(String::new());

    // Initialize bot instance
    let bot = Bot::new(token);

    // Initialize thread-safe set for active chat tracking
    let senders: Arc<Mutex<HashSet<i64>>> = Arc::new(Mutex::new(HashSet::new()));

    println!("Starting bot...");
    println!("GetMe status: {:?}", bot.get_me().await);

    // Initialize default handler
    let handler = get_storage_handler();

    // Initialize storage
    let storage = Arc::new(Mutex::new(storage::create_storage(db_enabled).await));

    // Start the dispatcher with configured dependencies
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![senders, storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
