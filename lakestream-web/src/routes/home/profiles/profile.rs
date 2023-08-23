use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use super::ProfileView;
use crate::components::forms::{
    ConfigurationFormMeta, FormStorageHandler, LocalStorageWrapper,
};
use crate::GlobalState;

#[component]
pub fn ProfileId(cx: Scope) -> impl IntoView {
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
    let form_meta_signal = create_rw_signal(cx, None::<ConfigurationFormMeta>);

    let error_signal = create_rw_signal(cx, None::<String>);

    let storage_wrapper = LocalStorageWrapper::new(vault);
    let storage_handler = FormStorageHandler::new(storage_wrapper);

    let storage_handler_clone = storage_handler.clone();
    spawn_local(async move {
        match storage_handler_clone.get_configuration_meta(&form_id).await {
            Ok(form_meta) => {
                form_meta_signal.set(Some(form_meta));
            }
            Err(e) => {
                log!("Error loading form_meta: {:?}", e);
                error_signal.set(Some(format!("{:?}", e)));
            }
        }
    });

    view! {
        cx,
        { move || if let Some(form_meta) = form_meta_signal.get() {
            view! {
                cx,
                <ProfileView storage_handler=storage_handler.clone() form_meta/>
            }.into_view(cx)
        } else if error_signal.get().is_some() {
            view! {
                cx,
                <div>
                <h1>"404: Page Not Found"</h1>
                 <p>"The page you requested could not be found."</p>
                </div>
            }.into_view(cx)
        } else {
            view! {
                cx,
                <div> { "Loading..." } </div> }.into_view(cx)
            }.into_view(cx)
        }
    }
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
