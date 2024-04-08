use std::future::Future;
use std::pin::Pin;

use crate::api::error::*;
use crate::api::handler::AppHandler;
use crate::impl_app_handler;

#[derive(Clone)]
pub struct Handler;

impl AppHandler for Handler {
    // mandatory boilerplate
    impl_app_handler!();

    fn invoke_main(
        &self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> {
        Box::pin(async move {
            println!("App initialized with args: {:?}", args);
            Ok(())
        })
    }
}
