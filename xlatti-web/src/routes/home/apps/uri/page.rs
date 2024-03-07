use leptos::*;

use crate::components::apps::configuration::AppConfigurationView;
use crate::components::apps::AppFormSubmit;

#[component]
pub fn AppConfiguration(cx: Scope) -> impl IntoView {
    // TODO: make app selectable
    let app_uri = "builtin::extract::objectstore".to_string();
    view! { cx,
        <AppConfigurationView app_uri=app_uri.clone()/>
        <AppFormSubmit app_uri=app_uri/>

    }
}
