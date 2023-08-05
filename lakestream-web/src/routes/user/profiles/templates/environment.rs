use crate::builders::{ElementBuilder, FieldBuilder, TextFieldBuilder};

pub fn form_elements<S: Into<String>>(name: S) -> Vec<ElementBuilder> {
    let builders: Vec<ElementBuilder> = vec![
        ElementBuilder::TextField(
            TextFieldBuilder::from(
                FieldBuilder::new("__NAME__").with_label("Name"),
            )
            .with_initial_value(name)
            .validator(None),
        ),
        ElementBuilder::TextField(
            TextFieldBuilder::from(
                FieldBuilder::new("Environment").with_label("Environment"),
            )
            .with_initial_value("auto")
            .validator(None),
        ),
    ];
    builders
}
