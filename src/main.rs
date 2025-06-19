//! Llama AI Telegram Bot
//!
//! Main application entry point that initializes and runs the Telegram bot
//! with Llama AI integration. Handles configuration loading and dispatcher setup.

use config::Config;
use dashmap::DashSet;
use lazy_static::lazy_static;
use std::sync::Arc;
use telegram::get_storage_handler;
use teloxide::prelude::*;

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
    println!("Preconfigure...");

    // Load bot token from configuration
    let token = CONFIG.get_string("token").unwrap_or(String::new());

    // Initialize bot instance
    let bot = Bot::new(token);

    println!("Starting bot...");
    println!("GetMe status: {:?}", bot.get_me().await);

    // Initialize default handler
    let handler = get_storage_handler();

    // Initialize storage
    let storage = storage::create_storage().await;

    let busy: Arc<DashSet<i64>> = Arc::new(DashSet::new());

    // Start the dispatcher with configured dependencies
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage, busy])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
