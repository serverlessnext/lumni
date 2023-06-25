use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use super::config_list::{Config, ConfigList};
use super::templates::{ConfigTemplate, Environment, ObjectStoreS3};
use crate::components::forms::{HtmlForm, SaveForm};
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
        .map(|route_params| route_params.id);

    let (is_loading, set_is_loading) = create_signal(cx, true);

    let form_loaded = create_rw_signal(cx, None::<HtmlForm>);

    let vault_clone = vault.clone();

    create_effect(cx, move |_| match form_id.as_ref() {
        Some(form_id) if !form_id.is_empty() => {
            if !form_id.is_empty() {
                let vault = vault_clone.clone();
                let form_loaded = form_loaded;
                let form_id = form_id.clone();
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

                        let form_data_option =
                            configurations.iter().find(|form_data| {
                                form_data.id() == form_id.as_str()
                            });

                        if let Some(form_data) = form_data_option {
                            let name = form_data.tags().and_then(|tags| {
                                tags.get("Name")
                                    .cloned()
                                    .or_else(|| Some("Untitled".to_string()))
                            });

                            // defaultts to Environment
                            let config_type = form_data
                                .tags()
                                .and_then(|tags| {
                                    tags.get("__CONFIGURATION_TYPE__").cloned()
                                })
                                .unwrap_or_else(|| "Environment".to_string());

                            if let Some(name) = name {
                                let config = match config_type.as_str() {
                                    "ObjectStoreS3" => Config::ObjectStoreS3(
                                        ObjectStoreS3::new(name.clone()),
                                    ),
                                    _ => Config::Environment(Environment::new(
                                        name.clone(),
                                    )),
                                };

                                let form_elements = config.form_elements(&name);
                                form_loaded.set(Some(HtmlForm::new(
                                    &name,
                                    form_id.as_str(),
                                    form_elements,
                                )));
                            }

                            set_is_loading.set(false);
                        }
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
                    let save_form = SaveForm::new(cx, form, &vault);
                    save_form.to_view()
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
