use leptos::*;
use uuid::Uuid;

use crate::builders::{build_all, FieldBuilder};
use crate::components::form_input::FormElement;
use crate::components::forms::{HtmlForm, SaveForm};
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
        FieldBuilder::new("field1").with_label("a").as_input_field(),
        FieldBuilder::new("field2").with_label("b").as_input_field(),
    ];

    let elements: Vec<FormElement> = build_all(builders);

    let form = HtmlForm::new(
        cx,
        &username,
        &Uuid::new_v4().to_string(),
        elements,
        None,
    );
    let save_form = SaveForm::new(cx, form, &vault);
    save_form.to_view()
}
