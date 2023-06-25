use crate::builders::{build_all, FieldBuilder, TextBoxBuilder};
use crate::components::form_input::FormElement;

pub fn form_elements<S: Into<String>>(name: S) -> Vec<FormElement> {
    let builders: Vec<TextBoxBuilder> = vec![
        TextBoxBuilder::from(FieldBuilder::new("__NAME__").with_label("Name"))
            .with_initial_value(name)
            .validator(None),
        TextBoxBuilder::from(
            FieldBuilder::new("Environment").with_label("Environment"),
        )
        .with_initial_value("auto")
        .validator(None),
    ];

    let elements: Vec<FormElement> = build_all(builders);
    elements
}
