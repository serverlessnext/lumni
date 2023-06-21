use leptos::*;
use uuid::Uuid;

use crate::components::form_input::{
    build_all, FieldBuilder, FormElement, InputFieldBuilder,
};
use crate::components::forms::{HtmlForm, SaveFormHandler};
use crate::GlobalState;

#[derive(Debug, PartialEq, Clone)]
pub struct RouteParams {
    id: String,
}

#[component]
pub fn UserSettings(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    // TODO: get this from vault
    let username = "admin".to_string();

    let builders = vec![
        InputFieldBuilder::from(FieldBuilder::new("field1").as_input_field())
            .default("".to_string()),
        InputFieldBuilder::from(FieldBuilder::new("field2").as_input_field())
            .default("".to_string()),
    ];

    let elements: Vec<FormElement> = build_all(builders);

    let form = HtmlForm::new(&username, &Uuid::new_v4().to_string(), elements);
    let save_form_handler = SaveFormHandler::new(cx, form, &vault);
    save_form_handler.create_view()
}
