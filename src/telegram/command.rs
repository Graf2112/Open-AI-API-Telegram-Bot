use crate::{
    storage::Storage, telegram::ai_request::handle_ai_request, telegram::message::BusySet,
};
use std::sync::Arc;
use teloxide::utils::command::BotCommands;
use teloxide::{prelude::*, types::Message, Bot};

/// Bot commands enumeration
///
/// Defines all available bot commands with their descriptions.
/// Uses lowercase naming convention for command matching.
#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    // Starts the bot and displays help message
    #[command(description = "bot conversation start. Use /help to get commands list.")]
    Start,
    // Displays list of available commands
    #[command(description = "displays description of all commands.")]
    Help,
    // Processes chat requests with Llama AI
    // Takes a String parameter containing the user's prompt
    #[command(description = "place your promt after this command. It will be sent to the model.")]
    Chat(String),
    // Clears conversation history
    #[command(description = "clears conversation context.")]
    Clear,
    // Sets system fingerprint for the model
    #[command(description = "set system fingerprint..")]
    System(String),
    // Sets temperature for the model
    #[command(description = "set temperature for model. Choose from 0.0 to 1.0. Default is 0.7.")]
    Temperature(f32),
    // Stops current operation
    #[command(description = "stops current operation.")]
    Stop,
}

/// Main command handler function
///
/// Processes incoming bot commands and returns appropriate responses
///
/// # Arguments
/// * `bot` - Telegram Bot instance
/// * `msg` - Incoming message containing the command
/// * `command` - Parsed command enum
/// * `senders` - Thread-safe set of chat IDs who await for the answer
///
/// # Returns
/// * `ResponseResult<()>` - Result of the command execution
pub async fn command_handler(
    bot: Bot,
    msg: Message,
    command: Command,
    busy: BusySet,
    storage: Arc<dyn Storage>,
) -> ResponseResult<()> {
    match command {
        Command::Start => {
            bot.send_message(msg.chat.id, "Welcome to Llama AI Telegram Bot!")
                .await?;
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Chat(text) => {
            let chat_id = msg.chat.id;
            let bot_clone = bot.clone();
            let storage_clone = storage.clone();
            let busy_clone = busy.clone();

            if msg.chat.is_channel() || msg.chat.is_group() || msg.chat.is_supergroup() {
                tokio::spawn(async move {
                    handle_ai_request(bot_clone, chat_id, text, storage_clone, busy_clone, true)
                        .await;
                });
            } else {
                tokio::spawn(async move {
                    handle_ai_request(bot_clone, chat_id, text, storage_clone, busy_clone, false)
                        .await;
                });
            }
        }
        Command::System(fingerprint) => {
            storage
                .set_system_fingerprint(msg.chat.id.0, fingerprint)
                .await;
            bot.send_message(msg.chat.id, "System fingerprint set")
                .await?;
        }
        Command::Temperature(temperature) => {
            let mut temperature = temperature as f32;

            if !{ 0.0..=1.0 }.contains(&temperature) {
                temperature = 0.7;
            }

            storage.set_temperature(msg.chat.id.0, temperature).await;
            bot.send_message(msg.chat.id, "Temperature set").await?;
        }
        Command::Clear => {
            storage.clear_conversation_context(msg.chat.id.0).await;
            bot.send_message(msg.chat.id, "Conversation cleared")
                .await?;
        }
        Command::Stop => {
            bot.send_message(msg.chat.id, "Stop").await?;
        }
    };
    Ok(())
}
