use std::sync::Arc;

use lumni::api::error::{ApplicationError, LumniError};
use lumni::api::spec::{
    parse_yaml_to_root, ApplicationSpec, SpecYamlType, YamlFormElement,
};
use regex::Regex;

use crate::components::forms::builders::ElementBuilder;

fn transform_to_configuration_environment(
    spec: &ApplicationSpec,
) -> Result<Vec<ElementBuilder>, LumniError> {
    if let Some(configuration) = spec.configuration() {
        form_elements_from_yaml(configuration.form_elements())
    } else {
        Err(LumniError::Application(
            ApplicationError::InvalidUserConfiguration(
                "Configuration not found in YAML.".to_string(),
            ),
            None,
        ))
    }
}

fn transform_to_interface_form(
    spec: &ApplicationSpec,
) -> Result<Vec<ElementBuilder>, LumniError> {
    if let Some(interface) = spec.interface() {
        form_elements_from_yaml(interface.form_elements())
    } else {
        Err(LumniError::Application(
            ApplicationError::InvalidUserConfiguration(
                "Interface not found in YAML.".to_string(),
            ),
            None,
        ))
    }
}

pub fn parse_yaml(
    yaml_str: &str,
    yaml_type: SpecYamlType,
) -> Result<Vec<ElementBuilder>, LumniError> {
    let root = parse_yaml_to_root(yaml_str)?;

    match yaml_type {
        SpecYamlType::Configuration => {
            transform_to_configuration_environment(&root)
        }
        SpecYamlType::Interface => transform_to_interface_form(&root),
        _ => Err(LumniError::Application(
            ApplicationError::InvalidUserConfiguration(
                "Invalid YAML type.".to_string(),
            ),
            None,
        )),
    }
}

fn form_elements_from_yaml(
    data: &[YamlFormElement],
) -> Result<Vec<ElementBuilder>, LumniError> {
    let mut results: Vec<ElementBuilder> = Vec::new();

    for element in data.iter() {
        let content_type = element.rtype().parse().unwrap_or_default();
        let mut builder = ElementBuilder::new(element.id(), content_type);

        if let Some(label_text) = element.label() {
            builder = builder.with_label(label_text);
        }

        if let Some(initial_value) = element.initial_value() {
            builder = builder.with_initial_value(initial_value);
        }

        if let Some(placeholder) = element.placeholder() {
            builder = builder.with_placeholder(placeholder);
        }

        if let Some(validation) = element.validation() {
            let pattern = match Regex::new(&validation.pattern()) {
                Ok(pat) => pat,
                Err(_) => {
                    return Err(LumniError::Application(
                        ApplicationError::InvalidUserConfiguration(
                            "Invalid regex pattern".into(),
                        ),
                        None,
                    ))
                }
            };
            let error_message = validation.error_message().to_string();
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
