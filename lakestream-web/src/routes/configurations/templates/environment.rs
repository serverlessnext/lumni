use crate::builders::{FieldBuilder, FieldBuilderTrait, TextBoxBuilder};

pub fn form_elements<S: Into<String>>(
    name: S,
) -> Vec<Box<dyn FieldBuilderTrait>> {
    let builders: Vec<Box<dyn FieldBuilderTrait>> = vec![
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("__NAME__").with_label("Name"),
            )
            .with_initial_value(name)
            .validator(None),
        ),
        Box::new(
            TextBoxBuilder::from(
                FieldBuilder::new("Environment").with_label("Environment"),
            )
            .with_initial_value("auto")
            .validator(None),
        ),
    ];

    builders
}
