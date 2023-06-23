use crate::components::form_input::{
    build_all, validate_with_pattern, FieldBuilder, FieldType, FormElement,
    TextBoxBuilder,
};

pub fn form_elements<S: Into<String>>(name: S) -> Vec<FormElement> {
    let builders: Vec<TextBoxBuilder> = vec![
        TextBoxBuilder::from(FieldBuilder::new("__NAME__").label("Name"))
            .default(name)
            .validator(None),
        TextBoxBuilder::from(
            FieldBuilder::new("Environment").label("Environment"),
        )
        .default("auto")
        .validator(None),
    ];

    let elements: Vec<FormElement> = build_all(builders);
    elements
}
