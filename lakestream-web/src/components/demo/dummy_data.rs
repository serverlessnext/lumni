use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;
use localencrypt::ItemMetaData;
use regex::Regex;

use crate::components::form_input::*;
use crate::components::forms::FormData;

pub fn make_form_elements() -> Vec<FormElement> {
    // textbox with validation
    let foo_pattern = Regex::new(r"^foo$").unwrap();
    let validate_foo = Some(validate_with_pattern_local(
        foo_pattern,
        "Input can only be foo".to_string(),
    ));

    let text_data_foo = TextData {
        field_type: FieldType::Text,
        field_label: Some(FieldLabel::new("Foo")),
        validator: validate_foo,
        buffer_data: "foo".to_string(),
    };

    let element_data_textbox = ElementData {
        name: "TextBoxElement".to_string(),
        element_type: ElementDataType::TextData(text_data_foo),
        is_enabled: true,
    };

    // textarea
    let text_data_any = TextData {
        field_type: FieldType::Text,
        field_label: Some(FieldLabel::new("Text Area")),
        validator: None,
        buffer_data: "type anything".to_string(),
    };
    let element_data_textarea = ElementData {
        name: "TextAreaElement".to_string(),
        element_type: ElementDataType::TextData(text_data_any),
        is_enabled: true,
    };

    let elements = vec![
        FormElement::TextBox(element_data_textbox),
        FormElement::TextArea(element_data_textarea),
    ];
    elements
}

pub fn make_form_data(cx: Scope) -> FormData {
    let elements = make_form_elements();
    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), "Test Form".to_string());
    let meta_data = ItemMetaData::new_with_tags("Form1", tags);
    let form_data = FormData::build(cx, meta_data, &elements);
    form_data
}

pub fn validate_with_pattern_local(
    pattern: Regex,
    error_msg: String,
) -> Arc<dyn Fn(&str) -> Result<(), String>> {
    let func = move |input: &str| {
        if pattern.is_match(input) {
            Ok(())
        } else {
            Err(error_msg.clone())
        }
    };
    Arc::new(func)
}
