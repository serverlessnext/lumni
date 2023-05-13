use leptos::*;

use crate::components::configuration_form::{ConfigurationForm, ConfigurationFormProps};

#[component]
pub fn Config(cx: Scope) -> impl IntoView {
    view! { cx,
        <h2>"Configuration S3 Bucket"</h2>
        <ConfigurationForm
            initial_config=vec![
                ("AWS_ACCESS_KEY_ID".to_string(), "".to_string()),
                ("AWS_SECRET_ACCESS_KEY".to_string(), "".to_string()),
                ("AWS_REGION".to_string(), "auto".to_string()),
                ("S3_ENDPOINT_URL".to_string(), "".to_string()),
            ]
        />
    }
}

