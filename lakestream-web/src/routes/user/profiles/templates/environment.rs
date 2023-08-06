use crate::builders::ElementBuilder;
use crate::components::form_input::FieldType;

pub fn form_elements<S: Into<String>>(name: S) -> Vec<ElementBuilder> {
    let builders: Vec<ElementBuilder> = vec![
        ElementBuilder::new("__NAME__", FieldType::Text)
            .with_label("Name")
            .with_initial_value(name)
            .validator(None),
        ElementBuilder::new("Environment", FieldType::Text)
            .with_label("Environment")
            .with_initial_value("auto")
            .validator(None),
    ];
    builders
}
