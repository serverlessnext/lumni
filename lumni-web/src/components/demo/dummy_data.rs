use std::collections::HashMap;
use std::sync::Arc;

use regex::Regex;

use crate::components::forms::input::*;
use crate::components::forms::{ConfigurationFormMeta, FormData};

pub fn make_form_elements() -> Vec<FormElement> {
    // textbox with validation
    let foo_pattern = Regex::new(r"^foo$").unwrap();
    let validate_foo = Some(validate_with_pattern_local(
        foo_pattern,
        "Input can only be foo".to_string(),
    ));

    let foo_element = FormElement {
        field_content_type: FieldContentType::PlainText,
        field_label: Some(FieldLabel::new("Foo")),
        field_placeholder: Some(FieldPlaceholder::new("foo")),
        validator: validate_foo,
        buffer_data: "foo".as_bytes().to_vec(),
        name: "TextBoxElement".to_string(),
        is_enabled: true,
    };

    let text_area_element = FormElement {
        field_content_type: FieldContentType::PlainText,
        field_label: Some(FieldLabel::new("Text Area")),
        field_placeholder: Some(FieldPlaceholder::new("key=value")),
        validator: None,
        buffer_data: "type anything".as_bytes().to_vec(),
        name: "TextAreaElement".to_string(),
        is_enabled: true,
    };

    let elements = vec![foo_element, text_area_element];
    elements
}

pub fn make_form_data() -> FormData {
    let elements = make_form_elements();
    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), "Test Form".to_string());

    let form_meta = ConfigurationFormMeta::with_id("Form1").with_tags(tags);
    let form_data = FormData::build(form_meta, &elements, None);
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
