use teloxide::{dispatching::{HandlerExt, UpdateFilterExt}, dptree::{self, Handler}, prelude::DependencyMap, types::Update};

use crate::telegram::storage_handler::{answer, invalid, message_handler, Command};

pub fn _get_default_handler() -> Handler<
    'static,
    DependencyMap,
    Result<(), teloxide::RequestError>,
    teloxide::dispatching::DpHandlerDescription,
    > {
    dptree::entry().branch(Update::filter_message().endpoint(invalid))
}

pub fn get_storage_handler() -> Handler<
    'static,
    DependencyMap,
    Result<(), teloxide::RequestError>,
    teloxide::dispatching::DpHandlerDescription,
> {
    dptree::entry()
        .branch(
            Update::filter_message()
                .branch(dptree::entry().filter_command::<Command>().endpoint(answer)),
        )
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_message().endpoint(invalid))
}