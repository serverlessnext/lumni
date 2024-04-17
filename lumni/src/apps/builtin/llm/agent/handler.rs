#[cfg(feature = "cli")]
use std::future::Future;
#[cfg(feature = "cli")]
use std::pin::Pin;

#[cfg(feature = "cli")]
use lumni::api::error::*;
use lumni::api::handler::AppHandler;

#[cfg(feature = "cli")]
use super::cli::run_cli;
use crate::{external as lumni, impl_app_handler};

#[derive(Clone)]
pub struct Handler;

impl AppHandler for Handler {
    // mandatory boilerplate
    impl_app_handler!();

    #[cfg(feature = "cli")]
    fn invoke_main(
        &self,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>>>> {
        Box::pin(async move {
            run_cli(args).await?;
            Ok(())
        })
    }
}
