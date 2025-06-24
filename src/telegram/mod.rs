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
    DependencyMap,
    Result<(), teloxide::RequestError>,
    teloxide::dispatching::DpHandlerDescription,
> {
    dptree::entry()
        .branch(
            Update::filter_message().branch(
                dptree::entry()
                    .filter_command::<Command>()
                    .endpoint(command_handler),
            ),
        )
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_inline_query().endpoint(inline_handler))
        .branch(Update::filter_message().endpoint(invalid))
}
