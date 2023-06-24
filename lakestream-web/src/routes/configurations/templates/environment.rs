use crate::components::form_input::{
    build_all, FieldBuilder, FormElement, TextBoxBuilder,
};

pub fn form_elements<S: Into<String>>(name: S) -> Vec<FormElement> {
    let builders: Vec<TextBoxBuilder> = vec![
        TextBoxBuilder::from(FieldBuilder::new("__NAME__").label("Name"))
            .with_initial_value(name)
            .validator(None),
        TextBoxBuilder::from(
            FieldBuilder::new("Environment").label("Environment"),
        )
        .with_initial_value("auto")
        .validator(None),
    ];

    let elements: Vec<FormElement> = build_all(builders);
    elements
}
