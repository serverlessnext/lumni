use super::field_builder::FieldBuilderTrait;
use super::text_box_builder::TextBoxBuilder;
use crate::components::form_input::FormElement;

pub enum ElementBuilder {
    TextBox(TextBoxBuilder),
}

impl From<TextBoxBuilder> for ElementBuilder {
    fn from(builder: TextBoxBuilder) -> Self {
        ElementBuilder::TextBox(builder)
    }
}

impl FieldBuilderTrait for ElementBuilder {
    fn build(&self) -> FormElement {
        match self {
            ElementBuilder::TextBox(builder) => builder.build(),
        }
    }

    fn box_clone(&self) -> Box<dyn FieldBuilderTrait> {
        match self {
            ElementBuilder::TextBox(builder) => Box::new(builder.clone()),
        }
    }
}
