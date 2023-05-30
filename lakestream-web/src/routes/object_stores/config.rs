use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use crate::components::forms::object_store::ObjectStore;
use crate::components::stringvault::FormHandler;
use crate::GlobalState;

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
pub fn ObjectStoresId(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let params = use_params::<RouteParams>(cx);
    // TODO: implement error handling
    let form_id: Option<String> = match params.try_get() {
        Some(Ok(route_params)) => Some(route_params.id.clone()),
        Some(Err(_)) => panic!("invalid params"),
        None => panic!("params not found"),
    };
    let form_id_clone = form_id.as_deref().unwrap_or_default().to_string();

    let (is_loading, set_is_loading) = create_signal(cx, true);
    let (config_name, set_config_name) = create_signal(cx, None::<String>);


    let vault_clone = vault.clone();
    create_effect(cx, move |_| {
        let vault = vault_clone.clone();
        spawn_local({
            let set_config_name = set_config_name.clone();
            let vault = vault.clone();
            let id = form_id.clone();

            async move {
                let configurations =
                    vault.list_configurations().await.unwrap_or_default();
                let name =
                    configurations.into_iter().find_map(|(config_id, name)| {
                        if config_id == id.as_deref().unwrap_or_default() {
                            Some(name)
                        } else {
                            None
                        }
                    });
                set_config_name.set(name);
                set_is_loading.set(false);
            }
        });
    });

    view! {
        cx,
        {move || if is_loading.get() {
            view! { cx, <div>"Loading..."</div> }
        } else {
            match config_name.get() {
                Some(name) => {
                    let config_manager = ObjectStore {
                        name: name.clone(),
                        id: form_id_clone.clone(),
                    };
                    let form_handler = FormHandler::new(config_manager, vault.clone());
                    form_handler.form_data_handler(cx)
                }
                None => {
                    view! {
                        cx,
                        <div>
                            <h1>"404: Page Not Found"</h1>
                            <p>"The page you requested could not be found."</p>
                        </div>
                    }
                }
            }
        }
    }}
}
