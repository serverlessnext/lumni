use std::future::Future;
use std::pin::Pin;

use futures::channel::mpsc;

use super::error::LumniError;
use super::invoke::Request;
use super::spec::ApplicationSpec;

pub trait AppHandler: Send + Sync + 'static {
    // handled by the macro impl_app_handler!()
    fn clone_box(&self) -> Box<dyn AppHandler>;
    fn load_specification(&self) -> &str;

    // methods the app can implement
    fn incoming_request(
        &self,
        _rx: mpsc::UnboundedReceiver<Request>,
    ) -> Pin<Box<dyn Future<Output = Result<(), LumniError>>>> {
        let package_name = self.package_name();
        Box::pin(async move {
            Err(LumniError::NotImplemented(format!(
                "Incoming request handling is not implemented for '{}'.",
                package_name
            )))
        })
    }

    fn invoke_main(
        &self,
        _spec: ApplicationSpec,
        _args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), LumniError>>>> {
        let package_name = self.package_name();
        Box::pin(async move {
            Err(LumniError::NotImplemented(format!(
                "CLI is not implemented for '{}'.",
                package_name
            )))
        })
    }

    // high-level functions
    // -- these should typically not need to reimplemented
    fn handle_query(
        &self,
        rx: mpsc::UnboundedReceiver<Request>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let (local_tx, local_rx) = futures::channel::oneshot::channel();
        let processing_future = self.incoming_request(rx);

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

    fn application_spec(&self) -> ApplicationSpec {
        serde_yaml::from_str::<ApplicationSpec>(self.load_specification())
            .expect("Failed to load application specification")
    }

    fn package_name(&self) -> String {
        let spec = self.application_spec();
        let package = spec.package();
        match package {
            Some(package) => package.name().to_string(),
            None => {
                // this should never happen as the spec is validated at compile time
                panic!("Failed to load package name from specification.");
            }
        }
    }

    fn package_version(&self) -> String {
        let spec = self.application_spec();
        let package = spec.package();
        match package {
            Some(package) => package.version().to_string(),
            None => {
                // this should never happen as the spec is validated at compile time
                panic!("Failed to load package version from specification.");
            }
        }
    }
}
