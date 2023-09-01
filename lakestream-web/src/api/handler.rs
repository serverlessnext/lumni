use std::future::Future;
use std::pin::Pin;

use futures::channel::mpsc;

use crate::api::invoke::Request;

pub trait AppHandler {
    fn handle_query(
        &self,
        rx: mpsc::UnboundedReceiver<Request>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}
