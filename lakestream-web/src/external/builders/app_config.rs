use std::fmt::Debug;
use std::sync::Arc;

use regex::Regex;
use serde::Deserialize;
use serde_yaml;
use uuid::Uuid;

use super::config_template::ConfigTemplate;
use crate::components::builders::ElementBuilder;

// TODO: config will be dynamically loaded from a file in future update
const OBJECT_STORE_S3_YAML: &str = r#"
elements:
  - id: "BUCKET_URI"
    type: "PlainText"
    label: "Bucket URI"
    initial_value: "s3://"
    validation:
      pattern: "^s3://"
      error_message: "Invalid URI scheme. Must start with 's3://'."

  - id: "AWS_ACCESS_KEY_ID"
    type: "PlainText"
    label: "AWS Access Key ID"
    validation:
      pattern: "^.+$"
      error_message: "Invalid AWS access key id."

  - id: "AWS_SECRET_ACCESS_KEY"
    type: "Secret"
    label: "AWS Secret Access Key"
    validation:
      pattern: "^.+$"
      error_message: "Invalid AWS secret access key."

  - id: "AWS_REGION"
    type: "PlainText"
    label: "AWS Region"
    initial_value: "auto"
    validation:
      pattern: "^[-a-zA-Z0-9]*$"
      error_message: "Invalid AWS region."

  - id: "S3_ENDPOINT_URL"
    type: "PlainText"
    label: "S3 Endpoint URL"
    validation:
      pattern: "^https?://[^/]+/$|^$"
      error_message: "Invalid S3 endpoint URL."
"#;

#[derive(Debug, Clone)]
pub struct Config {
    name: String,
    id: String,
    app_name: String,
}

impl ConfigTemplate for Config {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn app_name(&self) -> String {
        self.app_name.clone()
    }
}

impl FormElementList for Config {
    fn form_elements<S: Into<String>>(&self, _name: S) -> Vec<ElementBuilder> {
        // TODO: load yaml_string from app config file:
        // - apps/{app_name}/config/environment.yaml
        let yaml_string = OBJECT_STORE_S3_YAML;
        form_elements_from_yaml(yaml_string)
    }
}

pub trait FormElementList {
    fn form_elements<S: Into<String>>(&self, name: S) -> Vec<ElementBuilder>;
}

fn form_elements_from_yaml(yaml_string: &str) -> Vec<ElementBuilder> {
    let parsed_yaml: YamlStructure =
        serde_yaml::from_str(yaml_string).unwrap();
    let form_elements = parsed_yaml.elements;

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

#[derive(Debug, Deserialize)]
struct YamlStructure {
    elements: Vec<YamlElement>,
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

pub fn load_app_config(
    app_name: &str,
    profile_name: String,
    id: Option<String>,
) -> Config {
    Config {
        name: profile_name,
        id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        app_name: app_name.to_string(),
    }
}
