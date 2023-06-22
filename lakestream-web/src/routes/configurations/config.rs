use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use super::object_store_s3::ObjectStoreS3;
use crate::components::forms::{HtmlForm, SaveFormHandler};
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
pub fn ConfigurationId(cx: Scope) -> impl IntoView {
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

    let form_loaded = create_rw_signal(cx, None::<HtmlForm>);

    let vault_clone = vault.clone();
    let form_id_clone = form_id.clone();
    create_effect(cx, move |_| match form_id_clone.as_ref() {
        Some(form_id) if !form_id.is_empty() => {
            if !form_id.is_empty() {
                let vault = vault_clone.clone();
                let form_loaded = form_loaded.clone();
                let form_id = form_id_clone.clone();
                spawn_local({
                    async move {
                        let local_storage = match vault.backend() {
                            localencrypt::StorageBackend::Browser(
                                browser_storage,
                            ) => {
                                browser_storage.local_storage().unwrap_or_else(
                                    || panic!("Invalid browser storage type"),
                                )
                            }
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
                            //let object_store_form =
                            let default_fields =
                                ObjectStoreS3::default_fields(&name);
                            form_loaded.set(Some(HtmlForm::new(
                                &name,
                                &form_id.clone().unwrap_or_default(),
                                default_fields,
                            )));
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
            view! { cx, <div>"Loading..."</div> }.into_view(cx)
        } else {
            match form_loaded.get() {
                Some(form) => {
                    let save_form_handler = SaveFormHandler::new(cx, form, &vault);
                    save_form_handler.create_view()
                }
                None => {
                    view! {
                        cx,
                        <div>
                            <h1>"404: Page Not Found"</h1>
                            <p>"The page you requested could not be found."</p>
                        </div>
                    }.into_view(cx)
                }
            }
        }
    }}
}
