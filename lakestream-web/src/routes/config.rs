use leptos::*;

use crate::components::configuration_form::{
    ConfigurationFormLoader, ConfigurationFormLoaderProps,
};
use crate::components::login_form::{LoginForm, LoginFormProps};

#[component]
pub fn Config(cx: Scope) -> impl IntoView {
    view! { cx,
        <LoginForm />
        <h2>"Configuration S3 Bucket"</h2>
        <ConfigurationFormLoader
            initial_config=vec![
                ("AWS_ACCESS_KEY_ID".to_string(), "".to_string()),
                ("AWS_SECRET_ACCESS_KEY".to_string(), "".to_string()),
                ("AWS_REGION".to_string(), "auto".to_string()),
                ("S3_ENDPOINT_URL".to_string(), "".to_string()),
            ]
        />
    }
}
