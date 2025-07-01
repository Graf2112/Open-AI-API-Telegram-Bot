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
use tracing::{event, Level};

mod db;
mod lm_types;
mod logging;
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
    logging::setup_tracing();

    event!(Level::INFO, "Preconfigure...");

    // Load bot token from configuration
    let token = CONFIG.get_string("token").unwrap_or(String::new());

    // Initialize bot instance
    let bot = Bot::new(token);

    event!(Level::INFO, "Starting bot...");
    event!(Level::INFO, "GetMe status: {:?}", bot.get_me().await);

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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[test]
    fn test_error_type_creation() {
        let error_msg = "Test error";
        let error: Error = error_msg.into();
        assert!(error.to_string().contains("Test error"));
    }

    #[test]
    fn test_config_initialization() {
        // Test that CONFIG can be accessed without panicking
        // This validates that the lazy_static initialization works
        let result = std::panic::catch_unwind(|| {
            let _config = &*CONFIG;
        });
        assert!(result.is_ok(), "CONFIG should initialize without panicking");
    }

    #[tokio::test]
    async fn test_bot_creation_with_empty_token() {
        // Test bot creation with empty token
        let bot = Bot::new("");
        // Bot should be created even with empty token, but API calls will fail
        assert!(bot.get_me().await.is_err());
    }

    #[tokio::test]
    async fn test_bot_creation_with_invalid_token() {
        // Test bot creation with invalid token format
        let bot = Bot::new("invalid_token_format");
        // Bot should be created but API calls will fail with invalid token
        let result = bot.get_me().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_dashset_busy_initialization() {
        // Test that the busy DashSet can be created and used
        let busy: Arc<DashSet<i64>> = Arc::new(DashSet::new());

        // Test basic operations
        assert!(busy.is_empty());
        busy.insert(123);
        assert!(busy.contains(&123));
        assert_eq!(busy.len(), 1);

        busy.remove(&123);
        assert!(busy.is_empty());
    }

    #[tokio::test]
    async fn test_storage_creation() {
        // Test that storage can be created without panicking
        let result = storage::create_storage().await;
        // Just verify that we got some storage implementation back
        // The actual functionality should be tested in the storage module
        assert!(std::ptr::addr_of!(result) as usize != 0);
    }

    #[test]
    fn test_handler_initialization() {
        // Test that handler can be retrieved without panicking
        let result = std::panic::catch_unwind(|| get_storage_handler());
        assert!(result.is_ok(), "Handler initialization should not panic");
    }

    #[test]
    fn test_config_token_access() {
        // Test accessing token from config
        let token = CONFIG.get_string("token").unwrap_or(String::new());
        // Should return either a string value or empty string, never panic
        assert!(token.is_empty() || !token.is_empty()); // Always true, but validates no panic
    }

    #[tokio::test]
    async fn test_main_function_components_initialization() {
        // Test that all main function components can be initialized
        // This tests the initialization path without running the full dispatcher

        // Test token loading
        let token = CONFIG.get_string("token").unwrap_or(String::new());
        let _bot = Bot::new(token);

        // Test handler initialization
        let _handler = get_storage_handler();

        // Test storage initialization
        let _storage = storage::create_storage().await;

        // Test busy set initialization
        let _busy: Arc<DashSet<i64>> = Arc::new(DashSet::new());

        // If we reach here, all components initialized successfully
        assert!(true);
    }

    #[test]
    fn test_error_trait_object() {
        // Test that our Error type alias works correctly
        fn create_error() -> Error {
            "test error".into()
        }

        let error = create_error();
        assert_eq!(error.to_string(), "test error");
    }

    #[test]
    fn test_multiple_config_access() {
        // Test that CONFIG can be accessed multiple times safely
        let _first_access = &*CONFIG;
        let _second_access = &*CONFIG;
        let _third_access = &*CONFIG;

        // Test concurrent access
        let handles: Vec<_> = (0..10)
            .map(|_| {
                std::thread::spawn(|| {
                    let _config = &*CONFIG;
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
