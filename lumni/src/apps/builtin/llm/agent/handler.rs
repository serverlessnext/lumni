#[cfg(feature = "cli")]
use std::future::Future;
#[cfg(feature = "cli")]
use std::pin::Pin;

#[cfg(feature = "cli")]
use super::cli::run_prompter;
#[cfg(feature = "cli")]
use crate::api::error::*;
use crate::api::handler::AppHandler;
use crate::impl_app_handler;

#[derive(Clone)]
pub struct Handler;

impl AppHandler for Handler {
    // mandatory boilerplate
    impl_app_handler!();

    #[cfg(feature = "cli")]
    fn invoke_main(
        &self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> {
        Box::pin(async move {
            run_prompter(args).await?;
            Ok(())
        })
    }
}
