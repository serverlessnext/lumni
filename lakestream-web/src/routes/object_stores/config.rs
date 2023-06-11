use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use super::object_store::ObjectStore;
use crate::components::forms::form_handler::FormHandler;
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
    let form_id: Option<String> = params
        .try_get()
        .and_then(|result| result.ok())
        .map(|route_params| route_params.id.clone());

    let (is_loading, set_is_loading) = create_signal(cx, true);
    let is_object_store = create_rw_signal(cx, None::<ObjectStore>);

    let vault_clone = vault.clone();
    let form_id_clone = form_id.clone();
    create_effect(cx, move |_| match form_id_clone.as_ref() {
        Some(form_id) if !form_id.is_empty() => {
            if !form_id.is_empty() {
                let vault = vault_clone.clone();
                let is_object_store = is_object_store.clone();
                let form_id = form_id_clone.clone();
                spawn_local({

                    async move {
                        let local_storage = match vault.backend() {
                            localencrypt::StorageBackend::Browser(browser_storage) => {
                                browser_storage.local_storage().unwrap_or_else(|| panic!("Invalid browser storage type"))
                            },
                            _ => panic!("Invalid storage backend"),
                        };

                        let configurations = local_storage
                            .list_items()
                            .await
                            .unwrap_or_else(|_| vec![]);

                        let name = configurations
                            .iter()
                            .find(|form_data| {
                                form_data.id()
                                    == form_id.as_ref().unwrap().to_string()
                            })
                            .and_then(|form_data| {
                                form_data.tags().and_then(|tags| {
                                    tags.get("Name").cloned().or_else(|| {
                                        Some("Untitled".to_string())
                                    })
                                })
                            });

                        if let Some(name) = name {
                            is_object_store.set(Some(
                                ObjectStore::new_with_id(
                                    name.to_string(),
                                    form_id.clone().unwrap_or_default(),
                                ),
                            ));
                        }
                        set_is_loading.set(false);
                    }
                });
            }
        }
        _ => {
            set_is_loading.set(false);
        }
    });

    view! {
        cx,
        {move || if is_loading.get() {
            view! { cx, <div>"Loading..."</div> }
        } else {
            match is_object_store.get() {
                Some(object_store) => {
                    let form_handler = FormHandler::new(object_store, vault.clone());
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
