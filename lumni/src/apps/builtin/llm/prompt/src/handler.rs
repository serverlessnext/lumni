#[cfg(feature = "cli")]
use std::future::Future;
#[cfg(feature = "cli")]
use std::pin::Pin;

#[cfg(feature = "cli")]
use lumni::api::error::LumniError;
use lumni::api::handler::AppHandler;
#[cfg(feature = "cli")]
use lumni::api::{env::ApplicationEnv, spec::ApplicationSpec};

#[cfg(feature = "cli")]
use super::app::run_cli;
use crate::{external as lumni, impl_app_handler};

#[derive(Clone)]
pub struct Handler;

impl AppHandler for Handler {
    // mandatory boilerplate
    impl_app_handler!();

    #[cfg(feature = "cli")]
    fn invoke_main(
        &self,
        spec: ApplicationSpec,
        env: ApplicationEnv,
        args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), LumniError>>>> {
        Box::pin(async move {
            let app_name = spec.name();
            //run_cli(spec, args).await?;
            run_cli(spec, env, args).await.map_err(|e| {
                LumniError::Application(e, Some(app_name.clone()))
            })?;
            Ok(())
        })
    }
}
