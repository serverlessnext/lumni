use std::collections::HashMap;

use leptos::html::Div;
use leptos::*;
use uuid::Uuid;

use crate::components::form_input::{FormFieldBuilder, InputData};
use crate::components::forms::form_handler::{ConfigManager, FormHandler};
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

    let form_data_handler: HtmlElement<Div> = {
        let config_manager = UserForm::new(username);
        let form_handler = FormHandler::new(config_manager, vault);
        form_handler.form_data_handler(cx)
    };

    view! {
        cx,
        <div>
            {form_data_handler}
        </div>
    }
}

#[derive(Debug, Clone)]
pub struct UserForm {
    name: String,
    id: String,
}

impl UserForm {
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: Uuid::new_v4().to_string(),
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    fn default_fields(&self) -> HashMap<String, InputData> {
        vec![
            FormFieldBuilder::new("field1")
                .default("".to_string())
                .build(),
            FormFieldBuilder::new("field2")
                .default("".to_string())
                .build(),
        ]
        .into_iter()
        .map(|field| field.to_input_data())
        .collect()
    }
}

impl ConfigManager for UserForm {
    fn default_fields(&self) -> HashMap<String, InputData> {
        self.default_fields()
    }

    fn name(&self) -> String {
        self.name()
    }

    fn id(&self) -> String {
        self.id()
    }
}
