use crate::builders::ElementBuilder;
use crate::components::input::FieldContentType;

pub fn form_elements<S: Into<String>>(name: S) -> Vec<ElementBuilder> {
    let builders: Vec<ElementBuilder> = vec![
        ElementBuilder::new("__NAME__", FieldContentType::PlainText)
            .with_label("Name")
            .with_initial_value(name)
            .validator(None),
        ElementBuilder::new("Environment", FieldContentType::PlainText)
            .with_label("Environment")
            .with_initial_value("auto")
            .validator(None),
    ];
    builders
}
