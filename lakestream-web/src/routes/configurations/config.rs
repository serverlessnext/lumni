use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use super::configuration_view::{ConfigurationView, ConfigurationViewProps};
use crate::GlobalState;

#[component]
pub fn ConfigurationId(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let params = use_params::<RouteParams>(cx);
    let form_id: Option<String> = params
        .try_get()
        .and_then(|result| result.ok())
        .map(|route_params| route_params.id);
    let form_id = form_id.expect("form_id to be present");

    let props = ConfigurationViewProps { vault, form_id };
    ConfigurationView(cx, props)
}

#[derive(Debug, PartialEq, Clone)]
struct RouteParams {
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
