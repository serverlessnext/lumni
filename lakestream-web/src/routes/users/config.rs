use std::collections::HashMap;
use std::sync::Arc;
use regex::Regex;
use uuid::Uuid;

use leptos::html::Div;
use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use crate::GlobalState;
use crate::components::forms::helpers::validate_with_pattern;
use crate::components::forms::form_handler::{ConfigManager, InputData, FormInputFieldBuilder, FormHandler};


#[derive(Debug, PartialEq, Clone)]
pub struct RouteParams {
    id: String,
}

impl Params for RouteParams {
    fn from_map(map: &ParamsMap) -> Result<Self, ParamsError> {
        let id = map
            .get("id")
            .ok_or_else(|| ParamsError::MissingParam("id".to_string()))?;
        Ok(Self { id: id.to_string() })
    }
}

#[component]
pub fn UserId(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let params = use_params::<RouteParams>(cx);
    let id: Option<String> = match params.try_get() {
        Some(Ok(route_params)) => Some(route_params.id.clone()),
        Some(Err(_)) => None,
        None => None,
    };

    let form_data_handler: HtmlElement<Div> = match id {
        Some(id) if id == "admin" => {
            let config_manager = UserForm::new(id);
            let form_handler = FormHandler::new(config_manager, vault);
            form_handler.form_data_handler(cx)
        }
        _ => {
            // Render 404 page
            view! {
                cx,
                <div>
                    <h1>"404: Page Not Found"</h1>
                    <p>"The page you requested could not be found."</p>
                </div>
            }
            .into()
        }
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

    pub fn new_with_id(name: String, id: String) -> Self {
        Self { name, id }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    fn default_fields(&self) -> HashMap<String, InputData> {
        let password_pattern = Regex::new(r"^.{8,}$").unwrap();

        vec![FormInputFieldBuilder::new("PASSWORD")
            .default("".to_string())
            .password(true)
            .validator(Some(Arc::new(validate_with_pattern(
                password_pattern,
                "Invalid password. Must be at least 8 characters.".to_string(),
            ))))
            .build()]
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
