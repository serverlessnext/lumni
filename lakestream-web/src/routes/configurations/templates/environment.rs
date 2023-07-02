use crate::builders::{ElementBuilder, FieldBuilder, TextBoxBuilder};

pub fn form_elements<S: Into<String>>(
    name: S,
) -> Vec<ElementBuilder> {
    let builders: Vec<ElementBuilder> = vec![
        ElementBuilder::TextBox(
            TextBoxBuilder::from(
                FieldBuilder::new("__NAME__").with_label("Name"),
            )
            .with_initial_value(name)
            .validator(None)
        ),
        ElementBuilder::TextBox(
            TextBoxBuilder::from(
                FieldBuilder::new("Environment").with_label("Environment"),
            )
            .with_initial_value("auto")
            .validator(None)
        ),
    ];
    builders
}

