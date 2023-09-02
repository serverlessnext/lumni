use std::fmt::Debug;
use std::sync::Arc;

use regex::Regex;
use serde::Deserialize;
use serde_yaml;
use uuid::Uuid;

use super::get_app_handler;
use crate::api::handler::AppHandler;
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
        profile_name: S,
        profile_id: Option<S>,
    ) -> AppConfig {
        let app_uri = app_uri.into();
        let handler = get_app_handler(&app_uri).unwrap();
        // TODO: handle None

        AppConfig {
            app_uri,
            handler,
            profile_name: profile_name.into(),
            profile_id: profile_id
                .map_or_else(|| Uuid::new_v4().to_string(), |id| id.into()),
        }
    }

    fn load_config(&self) -> &str {
        self.handler.load_config()
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

    pub fn form_elements(&self) -> Vec<ElementBuilder> {
        let yaml_str = self.load_config();
        form_elements_from_yaml(yaml_str)
    }
}

fn form_elements_from_yaml(yaml_string: &str) -> Vec<ElementBuilder> {
    let parsed_yaml: Root = serde_yaml::from_str(yaml_string).unwrap();
    let form_elements = parsed_yaml.Configuration.Environment;

    form_elements
        .into_iter()
        .map(|element| {
            let content_type = element.r#type.parse().unwrap_or_default();
            let mut builder = ElementBuilder::new(&element.id, content_type);

            if let Some(label_text) = element.label {
                builder = builder.with_label(label_text);
            }

            if let Some(initial_value) = element.initial_value {
                builder = builder.with_initial_value(&initial_value);
            }

            if let Some(validation) = element.validation {
                let pattern = Regex::new(&validation.pattern).unwrap();
                builder =
                    builder.validator(Some(Arc::new(move |input: &str| {
                        if pattern.is_match(input) {
                            Ok(())
                        } else {
                            Err(validation.error_message.clone())
                        }
                    })));
            }

            builder
        })
        .collect()
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Root {
    Configuration: Configuration,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Configuration {
    Environment: Vec<YamlElement>,
}

#[derive(Debug, Deserialize)]
struct YamlElement {
    id: String,
    #[serde(rename = "type")]
    r#type: String,
    label: Option<String>,
    initial_value: Option<String>,
    validation: Option<YamlValidation>,
}

#[derive(Debug, Deserialize)]
struct YamlValidation {
    pattern: String,
    error_message: String,
}
