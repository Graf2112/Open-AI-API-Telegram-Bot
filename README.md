# Open-AI-API-Telegram-Bot

This repo had all to run telegram bot with in memory storage context and user settings.
This bot works with local language model. Tested via LM Studio.

# How to run

0. Install latest stable rust version
1. Create a telegram bot using [BotFather](https://t.me/BotFather)
2. Rename _settings.toml to settings.toml
3. Update settings.toml with your bot token
4. Update settings.toml with your url to local lm
5. Update settings.toml with your lm name
6. Run bot via 'cargo run'

# Basic usage

After running a bot, you can send a message to it.
Bot retrive a response from local language model and send it back to you or return error message.
Bot has in memory storage context and user settings. After restarting bot, all context and settings will be lost.
You can use /clear command to reset context and settings.

# Commands

- /start - start bot
- /help - show help message
- /clear - clear context and settings
- /system <Your system fingerpting> - show settings
- /temperature <0-1> - set temperature of language model
- /stop - stop previous response (Not working yet)