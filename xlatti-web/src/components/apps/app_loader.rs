use leptos::*;

use super::configuration::AppConfiguration;
use super::form_submit::AppFormSubmit;

#[component]
pub fn AppRunTime(cx: Scope, app_uri: String) -> impl IntoView {
    view! { cx,
        <AppFormSubmit app_uri/>
    }
}

#[component]
pub fn AppLoader(cx: Scope, app_uri: String) -> impl IntoView {
    // TODO:
    // AppConfigurationView should be put behind a toggable open/close link
    // add Logger that can be toggled open/close, this should show stdout/stderr

    view! { cx,
        <AppConfiguration app_uri=app_uri.clone()/>
        <AppRunTime app_uri/>
    }
}
