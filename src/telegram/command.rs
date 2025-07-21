use crate::storage::Note;
use crate::{
    storage::Storage, telegram::ai_request::handle_ai_request, telegram::message::BusySet,
};
use std::sync::Arc;
use teloxide::utils::command::BotCommands;
use teloxide::{Bot, prelude::*, types::Message};
use tracing::error;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum UserCommands {
    #[command(description = "bot conversation start. Use /help to get commands list.")]
    Start,
    #[command(description = "displays description of all commands.")]
    Help,
    #[command(description = "place your promt after this command. It will be sent to the model.")]
    Chat,
    #[command(description = "try to watch inyour future.")]
    Future,
}

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
    // // Stops current operation
    // #[command(description = "stops current operation.")]
    // Stop,
    #[command(description = "try to watch inyour future.")]
    Future,
    #[command(description = "add note.")]
    AddNote(String),
    #[command(description = "remove note.")]
    RemoveNote(i64),
    #[command(description = "list notes.")]
    ListNotes,
    #[command(description = "erase all notes.")]
    EraseNotes,
    #[command(description = "enable bot for this chat.")]
    Enable,
    #[command(description = "disable bot for this chat.")]
    Disable,
}

async fn is_admin(bot: &Bot, chat_id: ChatId, user_id: UserId) -> bool {
    match bot.get_chat_administrators(chat_id).await {
        Ok(admins) => admins.iter().any(|m| m.user.id == user_id),
        Err(_) => false,
    }
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
            bot.send_message(msg.chat.id, "Welcome to AI Telegram Bot!")
                .await?;
        }
        Command::Help => {
            if let Some(user) = msg.from {
                if !msg.chat.is_private() {
                    if is_admin(&bot, msg.chat.id, user.id).await {
                        bot.send_message(msg.chat.id, Command::descriptions().to_string())
                            .await?;
                    } else {
                        bot.send_message(msg.chat.id, UserCommands::descriptions().to_string())
                            .await?;
                    }
                } else if msg.chat.is_private() {
                    bot.send_message(msg.chat.id, Command::descriptions().to_string())
                        .await?;
                }
            }
        }
        Command::Chat(text) => {
            let message_id = msg.id;
            let chat_id = msg.chat.id;
            let bot_clone = bot.clone();
            let storage_clone = storage.clone();
            let busy_clone = busy.clone();

            if !msg.chat.is_private() {
                handle_ai_request(
                    bot_clone,
                    chat_id,
                    message_id,
                    text,
                    storage_clone,
                    busy_clone,
                )
                .await;
            } else {
                tokio::spawn(async move {
                    handle_ai_request(
                        bot_clone,
                        chat_id,
                        message_id,
                        text,
                        storage_clone,
                        busy_clone,
                    )
                    .await;
                });
            }
        }
        Command::System(fingerprint) => {
            if let Some(user) = msg.from {
                if !msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await {
                    bot.delete_message(msg.chat.id, msg.id).await?;
                    storage
                        .set_system_fingerprint(msg.chat.id.0, fingerprint)
                        .await;
                } else if msg.chat.is_private() {
                    storage
                        .set_system_fingerprint(msg.chat.id.0, fingerprint)
                        .await;
                    bot.send_message(msg.chat.id, "System fingerprint set")
                        .await?;
                }
            }
        }
        Command::Temperature(temperature) => {
            let mut temperature = temperature as f32;
            if !{ 0.0..=2.0 }.contains(&temperature) {
                temperature = 0.7;
            }
            if let Some(user) = msg.from {
                if !msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await {
                    bot.delete_message(msg.chat.id, msg.id).await?;
                    storage.set_temperature(msg.chat.id.0, temperature).await;
                } else if msg.chat.is_private() {
                    storage.set_temperature(msg.chat.id.0, temperature).await;
                    bot.send_message(msg.chat.id, "Temperature set").await?;
                }
            }
        }
        Command::Clear => {
            if let Some(user) = msg.from {
                if !msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await {
                    bot.delete_message(msg.chat.id, msg.id).await?;
                    storage.clear_conversation_context(msg.chat.id.0).await;
                } else if msg.chat.is_private() {
                    storage.clear_conversation_context(msg.chat.id.0).await;
                    bot.send_message(msg.chat.id, "Conversation cleared")
                        .await?;
                }
            }
        }
        Command::Future => {
            if let Some(user) = msg.from {
                let chat_id = msg.chat.id;
                let message_id = msg.id;
                let bot_clone = bot.clone();
                let storage_clone = storage.clone();
                let busy_clone = busy.clone();

                let promt = format!("Ты опытный предсказатель. Тебе нужно составить предсказание на день для человека. 
            Для гадания можешь на выбор использовать Таро, Руны или по звёздам. Текущая дата: {}
        Пользователь: {} Имя: {} Отвечай очень кратко.", chrono::Local::now(), user.username.clone().unwrap_or("Unknown".into()), user.full_name());
                handle_ai_request(
                    bot_clone,
                    chat_id,
                    message_id,
                    promt,
                    storage_clone,
                    busy_clone,
                )
                .await;
            }
        }
        Command::AddNote(text) => {
            if let Some(user) = msg.from {
                if !msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await {
                    let _ = bot.delete_message(msg.chat.id, msg.id).await;
                    storage
                        .add_note(Note {
                            note_id: chrono::Local::now().timestamp_millis(),
                            chat_id: msg.chat.id.0,
                            user_id: user.id.0,
                            text: text,
                        })
                        .await;
                } else if msg.chat.is_private() {
                    storage
                        .add_note(Note {
                            note_id: chrono::Local::now().timestamp_millis(),
                            chat_id: msg.chat.id.0,
                            user_id: user.id.0,
                            text: text,
                        })
                        .await;
                }
            }
        }
        Command::RemoveNote(id) => {
            if let Some(user) = msg.from {
                if !msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await {
                    let _ = bot.delete_message(msg.chat.id, msg.id).await;
                    storage.remove_note(msg.chat.id.0, id).await;
                } else if msg.chat.is_private() {
                    storage.remove_note(msg.chat.id.0, id).await;
                }
            }
        }
        Command::ListNotes => {
            if let Some(user) = msg.from {
                if (!msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await)
                    || msg.chat.is_private()
                {
                    if !msg.chat.is_private() {
                        let _ = bot.delete_message(msg.chat.id, msg.id).await;
                    }
                    let notes = storage.list_notes(msg.chat.id.0).await;
                    let mut ans = String::from("Notes for chat: \n");
                    for note in notes {
                        ans.push_str(&note.to_string());
                    }
                    #[allow(deprecated)]
                    if let Err(e) = bot
                        .send_message(user.id, &ans)
                        .parse_mode(teloxide::types::ParseMode::Markdown)
                        .await
                    {
                        if let Err(e) = bot.send_message(user.id, &ans).await {
                            error!("Failed to send message chunk to {}: {:?}", user.id, e);
                        }
                        error!("Something went wrong with Markdown {}: {:?}", user.id, e);
                    }
                }
            }
        }
        Command::EraseNotes => {
            if let Some(user) = msg.from {
                if (!msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await)
                    || msg.chat.is_private()
                {
                    storage.erase_notes(msg.chat.id.0).await;
                }
            }
        }
        Command::Enable => {
            if let Some(user) = msg.from {
                if (!msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await)
                    || msg.chat.is_private()
                {
                    if let Some(thread_id) = msg.thread_id {
                        storage
                            .enable(msg.chat.id.0, Some(thread_id.0.0 as i64))
                            .await;
                    } else {
                        storage.enable(msg.chat.id.0, None).await;
                    }
                }
            }
        }
        Command::Disable => {
            if let Some(user) = msg.from {
                if (!msg.chat.is_private() && is_admin(&bot, msg.chat.id, user.id).await)
                    || msg.chat.is_private()
                {
                    if let Some(thread_id) = msg.thread_id {
                        storage
                            .disable(msg.chat.id.0, Some(thread_id.0.0 as i64))
                            .await;
                    } else {
                        storage.disable(msg.chat.id.0, None).await;
                    }
                }
            }
        }
    };

    Ok(())
}
