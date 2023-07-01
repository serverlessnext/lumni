use std::fmt::Debug;

use uuid::Uuid;

use crate::builders::FieldBuilderTrait;

mod environment;
mod object_store_s3;

pub trait ConfigTemplate: CloneConfigTemplate + DebugConfigTemplate {
    fn new<S: Into<String>>(name: S) -> Self
    where
        Self: Sized;
    fn new_with_id<S: Into<String>>(name: S, id: S) -> Self
    where
        Self: Sized;
    fn name(&self) -> String;
    fn id(&self) -> String;
    fn template_name(&self) -> String;
}

pub trait CloneConfigTemplate {
    fn clone_box(&self) -> Box<dyn ConfigTemplate>;
}

impl<T> CloneConfigTemplate for T
where
    T: 'static + ConfigTemplate + Clone,
{
    fn clone_box(&self) -> Box<dyn ConfigTemplate> {
        Box::new(self.clone())
    }
}

pub trait DebugConfigTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<T> DebugConfigTemplate for T
where
    T: Debug + ConfigTemplate,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

pub trait FormElements {
    fn form_elements<S: Into<String>>(
        &self,
        name: S,
    ) -> Vec<Box<dyn FieldBuilderTrait>>;
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

            fn template_name(&self) -> String {
                stringify!($struct_name).to_string()
            }
        }

        impl FormElements for $struct_name {
            fn form_elements<S: Into<String>>(
                &self,
                name: S,
            ) -> Vec<Box<dyn FieldBuilderTrait>> {
                $default_fields_fn(name)
            }
        }
    };
}

impl_config_template!(ObjectStoreS3, object_store_s3::form_elements);
impl_config_template!(Environment, environment::form_elements);
