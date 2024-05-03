use serde::Deserialize;

use super::error::{ApplicationError, Error};

pub enum SpecYamlType {
    Package,
    Configuration,
    Interface,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct ApplicationSpec {
    package: Option<Package>,
    configuration: Option<Configuration>,
    interface: Option<Interface>,
}

impl ApplicationSpec {
    pub fn package(&self) -> Option<&Package> {
        self.package.as_ref()
    }

    pub fn configuration(&self) -> Option<&Configuration> {
        self.configuration.as_ref()
    }

    pub fn interface(&self) -> Option<&Interface> {
        self.interface.as_ref()
    }

    pub fn name(&self) -> String {
        self.package
            .as_ref()
            .map(|p| p.name())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn display_name(&self) -> String {
        self.package
            .as_ref()
            .map(|p| p.display_name())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn version(&self) -> String {
        self.package
            .as_ref()
            .map(|p| p.version())
            .unwrap_or_else(|| "0.0.0".to_string())
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Package {
    name: String,
    display_name: String,
    version: String,
}

impl Package {
    pub fn name(&self) -> String {
        // name is case-insensitive
        self.name.to_lowercase()
    }

    pub fn display_name(&self) -> String {
        self.display_name.clone()
    }

    pub fn version(&self) -> String {
        self.version.clone()
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Configuration {
    form_elements: Vec<YamlFormElement>,
}

impl Configuration {
    pub fn form_elements(&self) -> &Vec<YamlFormElement> {
        &self.form_elements
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Interface {
    form_elements: Vec<YamlFormElement>,
}

impl Interface {
    pub fn form_elements(&self) -> &Vec<YamlFormElement> {
        &self.form_elements
    }
}

#[derive(Debug, Deserialize)]
pub struct YamlFormElement {
    id: String,
    #[serde(rename = "type")]
    r#type: String,
    label: Option<String>,
    initial_value: Option<String>,
    placeholder: Option<String>,
    validation: Option<YamlValidation>,
}

impl YamlFormElement {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn rtype(&self) -> &str {
        &self.r#type
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn initial_value(&self) -> Option<&str> {
        self.initial_value.as_deref()
    }

    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    pub fn validation(&self) -> Option<&YamlValidation> {
        self.validation.as_ref()
    }
}

#[derive(Debug, Deserialize)]
pub struct YamlValidation {
    pattern: String,
    error_message: String,
}

impl YamlValidation {
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn error_message(&self) -> &str {
        &self.error_message
    }
}

pub fn parse_yaml_to_root(
    app_specification: &str,
) -> Result<ApplicationSpec, Error> {
    serde_yaml::from_str::<ApplicationSpec>(app_specification).map_err(|_| {
        Error::Application(ApplicationError::ConfigInvalid(
            "Failed to parse YAML".to_string(),
        ))
    })
}
