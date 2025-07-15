use command::{command_handler, Command};
use message::{invalid, message_handler};
use teloxide::{
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree::{self, Handler},
    prelude::DependencyMap,
    types::Update,
};

use crate::telegram::inline::inline_handler;

mod ai_request;
mod command;
mod inline;
mod message;

pub fn get_storage_handler() -> Handler<
    'static,
    Result<(), teloxide::RequestError>,
    teloxide::dispatching::DpHandlerDescription,
> {
    let command_branch = Update::filter_message()
    .filter_command::<Command>()
    .endpoint(command_handler);

    let message_branch = Update::filter_message().endpoint(message_handler);
    let inline_branch = Update::filter_inline_query().endpoint(inline_handler);
    let fallback = Update::filter_message().endpoint(invalid);

    dptree::entry()
        .branch(command_branch)
        .branch(message_branch)
        .branch(inline_branch)
        .branch(fallback)
}
