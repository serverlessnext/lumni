use std::fmt::Debug;
use std::sync::Arc;

use regex::Regex;
use serde::Deserialize;
use serde_yaml;
use uuid::Uuid;

use crate::components::builders::ElementBuilder;

#[derive(Debug, Clone)]
pub struct AppConfig {
    app_name: String,
    profile_name: String,
    profile_id: String,
    form_elements: Vec<ElementBuilder>,
}

impl AppConfig {
    pub fn new<S: Into<String>>(
        app_name: S,
        profile_name: S,
        profile_id: Option<S>,
    ) -> AppConfig {
        let app_name = app_name.into();
        let yaml_str = AppConfig::load_yaml_str(&app_name);
        let form_elements = form_elements_from_yaml(yaml_str);

        AppConfig {
            app_name,
            profile_name: profile_name.into(),
            profile_id: profile_id
                .map_or_else(|| Uuid::new_v4().to_string(), |id| id.into()),
            form_elements,
        }
    }

    fn load_yaml_str(_app_name: &str) -> &str {
        // TODO: in future update app will be loaded in memory
        // and not from the filesystem
        // the environment config should then be loaded from the app
        include_str!("../objectstore_s3/spec.yaml")
    }

    pub fn profile_name(&self) -> String {
        self.profile_name.clone()
    }

    pub fn profile_id(&self) -> String {
        self.profile_id.clone()
    }

    pub fn app_name(&self) -> String {
        self.app_name.clone()
    }

    pub fn form_elements(&self) -> Vec<ElementBuilder> {
        self.form_elements.clone()
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
