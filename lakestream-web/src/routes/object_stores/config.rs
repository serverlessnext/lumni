use std::collections::HashMap;

use leptos::*;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use crate::base::ObjectStoreList;
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
    let params = use_params::<RouteParams>(cx);

    let valid_ids: Vec<String> = ObjectStoreList::load_from_local_storage()
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
            view! {
                cx,
                <div>
                    <div>"You've requested object with ID: "{&id}</div>
                    <h2>"Configuration S3 Bucket"</h2>
                    <ObjectStoreConfig
                        uuid=id.clone()
                        initial_config={
                            let mut map = HashMap::new();
                            map.insert("AWS_ACCESS_KEY_ID".to_string(), "".to_string());
                            map.insert("AWS_SECRET_ACCESS_KEY".to_string(), "".to_string());
                            map.insert("AWS_REGION".to_string(), "auto".to_string());
                            map.insert("S3_ENDPOINT_URL".to_string(), "".to_string());
                            map
                        }
                    />
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
