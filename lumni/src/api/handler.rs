use std::future::Future;
use std::pin::Pin;

use futures::channel::mpsc;

use super::error::Error;
use super::invoke::Request;

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

        // Spawn logic for non-WASM32 architectures
        #[cfg(not(target_arch = "wasm32"))]
        tokio::task::spawn_local(async move {
            if let Err(e) = processing_future.await {
                log::error!("Error handling query: {:?}", e);
            }
            let _ = local_tx.send(());
        });

        // Spawn logic for WASM32 architecture
        #[cfg(target_arch = "wasm32")] 
        wasm_bindgen_futures::spawn_local(async move {
            log::info!("HALLASDASDSADDSADAS");
            let result = processing_future.await;
            if let Err(e) = result {
                log::error!("Error handling query: {:?}", e);
            }
            local_tx.send(()).expect("Failed to send completion signal");
        });

        Box::pin(async move {
            local_rx.await.expect("Failed to receive completion signal");
        })
    }

    fn load_config(&self) -> &str;
}
