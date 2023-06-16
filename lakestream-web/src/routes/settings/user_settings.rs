use leptos::*;
use uuid::Uuid;

use crate::components::form_input::FormFieldBuilder;
use crate::components::forms::{HtmlForm, HtmlFormHandler};
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

    let fields = vec![
        FormFieldBuilder::new("field1")
            .default("".to_string())
            .build(),
        FormFieldBuilder::new("field2")
            .default("".to_string())
            .build(),
    ]
    .into_iter()
    .map(|field| field.to_input_data())
    .collect();

    let form = HtmlForm::new(&username, &Uuid::new_v4().to_string(), fields);
    let form_handler = HtmlFormHandler::new(form, vault);
    form_handler.create_view(cx)
}
