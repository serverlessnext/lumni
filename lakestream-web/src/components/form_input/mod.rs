mod field_type;
mod form_element;
mod form_field_builder;
mod helpers;
mod text_box_view;
mod text_box;
mod text_box_builder;
mod text_area;
mod text_area_builder;

pub use field_type::FieldType;
pub use form_element::{FieldLabel, FormElement};
pub use form_field_builder::{
    build_all, FieldBuilder, FieldBuilderTrait,
};

pub use text_box_builder::{InputFieldPattern, TextBoxBuilder};
pub use helpers::validate_with_pattern;
pub use text_box_view::TextBoxView;
pub use text_box::{
    InputElement, InputElements, TextBox,
};
pub use text_area::TextArea;
