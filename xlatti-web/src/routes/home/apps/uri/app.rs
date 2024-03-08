use leptos::*;
use leptos::logging::log;
use leptos_router::{use_params, Params, ParamsError, ParamsMap};

use crate::components::apps::AppConfigView;
use crate::components::forms::ConfigurationFormMeta;
use crate::helpers::local_storage::create_storage_handler;

#[component]
pub fn AppId() -> impl IntoView {
    let params = use_params::<RouteParams>();
    let form_id: Option<String> = params
        .try_get()
        .and_then(|result| result.ok())
        .map(|route_params| route_params.id);
    let form_id = form_id.expect("form_id to be present");
    let form_meta_signal = create_rw_signal(None::<ConfigurationFormMeta>);

    let error_signal = create_rw_signal(None::<String>);

    // TODO: handle expect error via error_signal
    let storage_handler =
        create_storage_handler().expect("storage_handler to be present");

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
        { move || if let Some(form_meta) = form_meta_signal.get() {
            view! {
                <AppConfigView storage_handler=storage_handler.clone() form_meta/>
            }.into_view()
        } else if error_signal.get().is_some() {
            view! {
                <div>
                <h1>"404: Page Not Found"</h1>
                 <p>"The page you requested could not be found."</p>
                </div>
            }.into_view()
        } else {
            view! {
                <div> { "Loading..." } </div> }.into_view()
            }.into_view()
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
