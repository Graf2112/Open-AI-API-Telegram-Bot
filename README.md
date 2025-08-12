# Open-AI-API-Telegram-Bot (NOW UNDER RECONSTRUCTION)

This repo had everything to run a telegram bot, with in-memory or SQLite storage for context and user settings.
This bot works with a local language model, tested via LM Studio's LLMs.

# How to run

0. Install the latest stable version of Rust.
1. Create a telegram bot using [BotFather](https://t.me/BotFather)
2. Rename _settings.toml to settings.toml
3. Update settings.toml with your bot token
4. Update settings.toml with your url to local lm
5. Update settings.toml with your lm name
6. Set the db variable to true if you want to use SQLite storage, or false if not.
7. Run bot via 'cargo run'

# Basic usage

After running a bot, you can send it a message.
The bot retrieves a response from the local language model and sends it back to the user or returns an error message.
Bot has in memory or SQLite storage context and user settings. After restarting bot, all context and settings will be lost if SQLite not used.
You can use /clear command to reset context.

# Commands
In your telegram bot, use the following commands:
- /start - start bot
- /help - show help message
- /clear - clear context and settings
- /system Place here your system fingerprint - set your system fingerprint. This fingerprint will be used in every response.
- /temperature 0.0-1.0 - set temperature of language model in range 0.0-1.0
- /stop - stop previous response (Not working yet)
