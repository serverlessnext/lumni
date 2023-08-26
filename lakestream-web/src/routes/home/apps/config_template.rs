use std::fmt::Debug;

use crate::builders::ElementBuilder;

pub trait ConfigTemplate: CloneConfigTemplate + DebugConfigTemplate {
    fn name(&self) -> String;
    fn id(&self) -> String;
    fn app_name(&self) -> String;
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

pub trait FormElementList {
    fn form_elements<S: Into<String>>(&self, name: S) -> Vec<ElementBuilder>;
}
