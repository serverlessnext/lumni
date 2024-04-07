use std::sync::Arc;

use lumni::api::error::{ApplicationError, Error};
use regex::Regex;
use serde::Deserialize;

use crate::components::forms::builders::ElementBuilder;

pub enum ConfigYamlType {
    ConfigurationEnvironment,
    InterfaceForm,
}

pub fn parse_yaml(
    yaml_str: &str,
    yaml_type: ConfigYamlType,
) -> Result<Vec<ElementBuilder>, Error> {
    let root: Root = serde_yaml::from_str(yaml_str).map_err(|_| {
        Error::Application(ApplicationError::ConfigInvalid(
            "Failed to parse YAML".to_string(),
        ))
    })?;

    match yaml_type {
        ConfigYamlType::ConfigurationEnvironment => {
            if let Some(configuration) = root.configuration {
                form_elements_from_yaml(configuration.get_elements())
            } else {
                Err(Error::Application(ApplicationError::ConfigInvalid(
                    "Configuration not found in YAML.".to_string(),
                )))
            }
        }
        ConfigYamlType::InterfaceForm => {
            if let Some(interface) = root.interface {
                form_elements_from_yaml(interface.get_elements())
            } else {
                Err(Error::Application(ApplicationError::ConfigInvalid(
                    "Interface not found in YAML.".to_string(),
                )))
            }
        }
    }
}

trait ElementContainer {
    fn get_elements(&self) -> &Vec<YamlElement>;
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Root {
    configuration: Option<Configuration>,
    interface: Option<Interface>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Configuration {
    environment: Vec<YamlElement>,
}

impl ElementContainer for Configuration {
    fn get_elements(&self) -> &Vec<YamlElement> {
        &self.environment
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Interface {
    form: Vec<YamlElement>,
}

impl ElementContainer for Interface {
    fn get_elements(&self) -> &Vec<YamlElement> {
        &self.form
    }
}

#[derive(Debug, Deserialize)]
struct YamlElement {
    id: String,
    #[serde(rename = "type")]
    r#type: String,
    label: Option<String>,
    initial_value: Option<String>,
    placeholder: Option<String>,
    validation: Option<YamlValidation>,
}

#[derive(Debug, Deserialize)]
struct YamlValidation {
    pattern: String,
    error_message: String,
}

fn form_elements_from_yaml(
    data: &[YamlElement],
) -> Result<Vec<ElementBuilder>, Error> {
    let mut results: Vec<ElementBuilder> = Vec::new();

    for element in data.iter() {
        let content_type = element.r#type.parse().unwrap_or_default();
        let mut builder = ElementBuilder::new(&element.id, content_type);

        if let Some(label_text) = &element.label {
            builder = builder.with_label(label_text);
        }

        if let Some(initial_value) = &element.initial_value {
            builder = builder.with_initial_value(initial_value);
        }

        if let Some(placeholder) = &element.placeholder {
            builder = builder.with_placeholder(placeholder);
        }

        if let Some(validation) = &element.validation {
            let pattern = match Regex::new(&validation.pattern) {
                Ok(pat) => pat,
                Err(_) => {
                    return Err(Error::Application(
                        ApplicationError::ConfigInvalid(
                            "Invalid regex pattern".into(),
                        ),
                    ))
                }
            };
            let error_message = validation.error_message.clone();
            builder = builder.validator(Some(Arc::new(move |input: &str| {
                if pattern.is_match(input) {
                    Ok(())
                } else {
                    Err(error_message.clone())
                }
            })));
        }

        results.push(builder);
    }

    Ok(results)
}
