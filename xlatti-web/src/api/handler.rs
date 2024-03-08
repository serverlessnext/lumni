use std::future::Future;
use std::pin::Pin;

use futures::channel::mpsc;
use leptos::logging::error;

use crate::api::error::Error;
use crate::api::invoke::Request;

pub trait AppHandler: Send + Sync + 'static {
    fn clone_box(&self) -> Box<dyn AppHandler>;
    fn process_request(
        &self,
        rx: mpsc::UnboundedReceiver<Request>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>>>>;

    fn handle_query(
        &self,
        rx: mpsc::UnboundedReceiver<Request>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let (local_tx, local_rx) = futures::channel::oneshot::channel();
        let processing_future = self.process_request(rx);

        wasm_bindgen_futures::spawn_local(async move {
            let result = processing_future.await;
            if let Err(e) = result {
                error!("Error handling query: {:?}", e);
            }
            local_tx.send(()).expect("Failed to send completion signal");
        });

        Box::pin(async move {
            local_rx.await.expect("Failed to receive completion signal");
        })
    }

    fn load_config(&self) -> &str;
}
