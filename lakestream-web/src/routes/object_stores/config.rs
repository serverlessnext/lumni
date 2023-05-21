
use uuid::Uuid;
use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use crate::GlobalState;
use crate::base::{ObjectStore, ObjectStoreList};
use crate::components::configuration_form::ObjectStoreConfig;

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

    let valid_ids: Vec<String> = ObjectStoreList::load_from_local_storage(vault.clone())
        .into_iter()
        .map(|item| item.id.to_string())
        .collect();

    let id: Option<String> = match params.try_get() {
        Some(Ok(route_params)) => Some(route_params.id.clone()),
        Some(Err(_)) => None,
        None => None,
    };

    match id {
        Some(id) if valid_ids.contains(&id) => {
            let store = ObjectStore::new(
                Uuid::parse_str(&id).unwrap(),
                "s3://my-bucket".to_string(),
                vault,
            );
            view! {
                cx,
                <div>
                    <div>"You've requested object with ID: "{&id}</div>
                    <h2>"Configuration S3 Bucket"</h2>
                    <ObjectStoreConfig store=store/>
                </div>
            }
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
        }
    }
}
