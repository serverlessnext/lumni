use super::templates::*;
use crate::components::form_input::FormElement;

pub trait ConfigList {
    fn name(&self) -> String;
    fn id(&self) -> String;
    fn form_elements<S: Into<String>>(&self, name: S) -> Vec<FormElement>;
}

pub enum Config {
    ObjectStoreS3(ObjectStoreS3),
    Environment(Environment),
}

impl ConfigList for Config {
    fn name(&self) -> String {
        match self {
            Config::ObjectStoreS3(c) => c.name(),
            Config::Environment(c) => c.name(),
        }
    }

    fn id(&self) -> String {
        match self {
            Config::ObjectStoreS3(c) => c.id(),
            Config::Environment(c) => c.id(),
        }
    }

    fn form_elements<S: Into<String>>(&self, name: S) -> Vec<FormElement> {
        match self {
            Config::ObjectStoreS3(c) => c.form_elements(name),
            Config::Environment(c) => c.form_elements(name),
        }
    }
}
