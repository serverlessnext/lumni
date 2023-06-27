use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use uuid::Uuid;

use localencrypt::ItemMetaData;

use crate::builders::{
    FieldBuilder, FormBuilder, FormLoadParameters, FormType,
};

use crate::components::form_input::*;

use crate::components::forms::{FormData, FormError};

#[cfg(debug_assertions)]
#[cfg(feature = "debug-assertions")]
async fn debug_sleep() {
    use std::time::Duration;

    #[cfg(feature = "debug-assertions")]
    use async_std::task;
    task::sleep(Duration::from_secs(1)).await;
}

#[cfg(feature = "debug-assertions")]
macro_rules! debug_sleep {
    () => {
        #[cfg(debug_assertions)]
        {
            debug_sleep().await;
        }
    };
}


#[component]
pub fn LoadDemo(cx: Scope) -> impl IntoView {
    let is_loading = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    // define a function that fetches the data
    let handle_load = {
        let dummy_data = dummy_form_data(cx);
        move |form_data_rw: RwSignal<Option<FormData>>| {
            let dummy_data = dummy_data.clone();
            //is_loading.set(true);
            spawn_local(async move {
                // run data loading on the background
                let data = load_data().await;
                log!("Loaded data: {:?}", data);
                form_data_rw.set(Some(dummy_data));
                is_loading.set(false);
            });
        }
    };

    let load_parameters = FormLoadParameters::new(
        Some(Box::new(handle_load)),
        Some(is_loading),
        Some(validation_error),
    );

    let load_form = FormBuilder::new("Load Form", &Uuid::new_v4().to_string(), FormType::Load(Some(load_parameters)))
        .build(cx);

    load_form.to_view()
}


async fn load_data() -> Result<HashMap<String, String>, FormError> {
    #[cfg(feature = "debug-assertions")]
    debug_sleep!();

    Ok(HashMap::new())
}


fn dummy_form_data(cx: Scope) -> FormData {
    let text_data = TextData {
        field_type: FieldType::Text,
        field_label: Some(FieldLabel::new("Text Field")),
        validator: None,
        buffer_data: "Dummy Text".to_string(),
    };

//    let binary_data = BinaryData {
//        field_label: Some(FieldLabel::new("Binary Field")),
//        buffer_data: vec![1, 2, 3],
//    };
//
//    let document_data = DocumentData {
//        document_type: DocumentType::Html,
//        field_label: Some(FieldLabel::new("Document Field")),
//        validator: None,
//        buffer_data: "Initial Document Data".to_string(),
//    };

    let element_data_textbox = ElementData {
        name: "TextBoxElement".to_string(),
        element_type: ElementDataType::TextData(text_data.clone()),
        is_enabled: true,
    };

    let element_data_textarea = ElementData {
        name: "TextAreaElement".to_string(),
        element_type: ElementDataType::TextData(text_data),
        is_enabled: true,
    };

//    let element_data_nestedform = ElementData {
//        name: "NestedFormElement".to_string(),
//        element_type: ElementDataType::DocumentData(document_data),
//        is_enabled: true,
//    };

    let elements = vec![
        FormElement::TextBox(element_data_textbox),
        FormElement::TextArea(element_data_textarea),
        //FormElement::NestedForm(element_data_nestedform),
    ];

    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), "Test Form".to_string());
    let meta_data = ItemMetaData::new_with_tags("Form1", tags);

    let form_data = FormData::build(cx, meta_data, &elements);
    form_data
}
