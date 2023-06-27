use uuid::Uuid;

use crate::components::form_input::FormElement;

mod environment;
mod object_store_s3;

pub trait ConfigTemplate {
    fn new<S: Into<String>>(name: S) -> Self;
    fn new_with_id<S: Into<String>>(name: S, id: S) -> Self;
    fn name(&self) -> String;
    fn id(&self) -> String;
    fn form_elements<S: Into<String>>(&self, name: S) -> Vec<FormElement>;
}

macro_rules! impl_config_template {
    ($struct_name:ident, $default_fields_fn:expr) => {
        #[derive(Debug, Clone)]
        pub struct $struct_name {
            name: String,
            id: String,
        }

        impl ConfigTemplate for $struct_name {
            fn new<S: Into<String>>(name: S) -> Self {
                Self {
                    name: name.into(),
                    id: Uuid::new_v4().to_string(),
                }
            }

            fn new_with_id<S: Into<String>>(name: S, id: S) -> Self {
                Self {
                    name: name.into(),
                    id: id.into(),
                }
            }

            fn name(&self) -> String {
                self.name.clone()
            }

            fn id(&self) -> String {
                self.id.clone()
            }

            fn form_elements<S: Into<String>>(
                &self,
                name: S,
            ) -> Vec<FormElement> {
                $default_fields_fn(name)
            }
        }
    };
}

impl_config_template!(ObjectStoreS3, object_store_s3::form_elements);
impl_config_template!(Environment, environment::form_elements);
