use handler::{answer, invalid, message_handler, Command};
use teloxide::{dispatching::{HandlerExt, UpdateFilterExt}, dptree::{self, Handler}, prelude::DependencyMap, types::Update};

pub mod handler;

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