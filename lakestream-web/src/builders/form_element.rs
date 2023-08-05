use super::field_builder::FieldBuilderTrait;
use super::text_field_builder::TextFieldBuilder;
use crate::components::form_input::FormElement;

pub enum ElementBuilder {
    TextField(TextFieldBuilder),
}

impl From<TextFieldBuilder> for ElementBuilder {
    fn from(builder: TextFieldBuilder) -> Self {
        ElementBuilder::TextField(builder)
    }
}

impl FieldBuilderTrait for ElementBuilder {
    fn build(&self) -> FormElement {
        match self {
            ElementBuilder::TextField(builder) => builder.build(),
        }
    }

    fn box_clone(&self) -> Box<dyn FieldBuilderTrait> {
        match self {
            ElementBuilder::TextField(builder) => Box::new(builder.clone()),
        }
    }
}
