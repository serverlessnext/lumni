use std::future::Future;
use std::pin::Pin;

use futures::channel::mpsc;
use futures::stream::StreamExt;

use crate::api::error::*;
use crate::api::handler::AppHandler;
use crate::api::invoke::{Request, Response};
use crate::api::types::Data;

#[derive(Clone)]
pub struct Handler;

impl AppHandler for Handler {
    fn clone_box(&self) -> Box<dyn AppHandler> {
        Box::new(self.clone())
    }
    fn process_request(
        &self,
        rx: mpsc::UnboundedReceiver<Request>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>>>> {
        Box::pin(handle_query(rx))
    }

    fn handle_runtime(
        &self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> {
        Box::pin(async move {
            println!("App initialized with args: {:?}", args);
            Ok(())
        })
    }

    fn load_config(&self) -> &str {
        include_str!("spec.yaml")
    }
}

pub async fn handle_query(
    mut rx: mpsc::UnboundedReceiver<Request>,
) -> Result<(), Error> {
    if let Some(request) = rx.next().await {
        let tx = request.tx();

        let response = Response::new(Data::Empty);
        tx.unbounded_send(Ok(response)).unwrap();
    }
    Ok(())
}
