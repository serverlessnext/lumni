use std::fmt::Debug;

use lumni::api::error::Error;
use lumni::api::get_app_handler;
use lumni::api::handler::AppHandler;
use lumni::api::spec::SpecYamlType;
use uuid::Uuid;

use super::parse_config::parse_yaml;
use crate::components::forms::builders::ElementBuilder;

pub struct AppConfig {
    app_uri: String,
    handler: Box<dyn AppHandler>,
    profile_name: String,
    profile_id: String,
}

impl Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("app_uri", &self.app_uri)
            .field("profile_name", &self.profile_name)
            .field("profile_id", &self.profile_id)
            .finish()
    }
}

impl Clone for AppConfig {
    fn clone(&self) -> Self {
        AppConfig {
            app_uri: self.app_uri.clone(),
            handler: self.handler.clone_box(),
            profile_name: self.profile_name.clone(),
            profile_id: self.profile_id.clone(),
        }
    }
}

impl AppConfig {
    pub fn new<S: Into<String>>(
        app_uri: S,
        profile_name: Option<S>,
        profile_id: Option<S>,
    ) -> Option<AppConfig> {
        let app_uri = app_uri.into();
        let handler = match get_app_handler(&app_uri) {
            Some(handler) => handler,
            None => return None,
        };

        let profile_name =
            profile_name.map_or_else(|| "Untitled".to_string(), Into::into);
        let profile_id =
            profile_id.map_or_else(|| Uuid::new_v4().to_string(), Into::into);

        Some(AppConfig {
            app_uri,
            handler,
            profile_name: profile_name.into(),
            profile_id,
        })
    }

    fn load_app_specification(&self) -> &str {
        self.handler.load_specification()
    }

    pub fn handler(&self) -> &dyn AppHandler {
        self.handler.as_ref()
    }

    pub fn profile_name(&self) -> String {
        self.profile_name.clone()
    }

    pub fn profile_id(&self) -> String {
        self.profile_id.clone()
    }

    pub fn app_uri(&self) -> String {
        self.app_uri.clone()
    }

    pub fn configuration_form_elements(
        &self,
    ) -> Result<Vec<ElementBuilder>, Error> {
        parse_yaml(self.load_app_specification(), SpecYamlType::Configuration)
    }

    pub fn interface_form_elements(
        &self,
    ) -> Result<Vec<ElementBuilder>, Error> {
        parse_yaml(self.load_app_specification(), SpecYamlType::Interface)
    }
}

//async fn load_config(cx: Scope, form_id: String) -> Result<(), Error> {
//    log!("Loading config");
//    let storage_handler = create_storage_handler(cx);
//    match storage_handler {
//        Some(handler) => {
//            let items = handler.list_items().await;
//            log!("Items: {:?}", items);
//            let result = handler.load_config(&form_id).await;
//            log!("Result: {:?}", result);
//        }
//        None => {
//            log!("Storage handler is not available");
//        }
//    }
//   Ok(())
//}
