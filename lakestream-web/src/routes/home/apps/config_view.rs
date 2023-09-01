use leptos::ev::SubmitEvent;
use leptos::*;
use leptos_router::use_query_map;

use super::AppConfig;
use crate::components::buttons::{ButtonType, FormButton};
use crate::components::forms::builders::{
    FormType, LoadParameters, ProfileFormBuilder, SubmitParameters,
};
use crate::components::forms::input::perform_validation;
use crate::components::forms::{
    ConfigurationFormMeta, FormData, FormStorageHandler, LocalStorageWrapper,
};

#[component]
pub fn AppConfigView(
    cx: Scope,
    storage_handler: FormStorageHandler<LocalStorageWrapper>,
    form_meta: ConfigurationFormMeta,
) -> impl IntoView {
    let is_loading = create_rw_signal(cx, false);
    let load_error = create_rw_signal(cx, None::<String>);

    let form_id_clone = form_meta.id();
    let storage_handler_clone = storage_handler.clone();

    let param_view = use_query_map(cx).get().get("view").cloned();
    let is_text_area = param_view
        .as_ref()
        .map(|v| v.as_str() == "TextArea")
        .unwrap_or(false);

    let handle_load = move |form_data_rw: RwSignal<Option<FormData>>| {
        let form_id = form_id_clone.to_owned();
        let storage_handler = storage_handler_clone.to_owned();
        is_loading.set(true);
        spawn_local(async move {
            match storage_handler.load_config(&form_id).await {
                Ok(Some(config)) => {
                    log!("Data loaded for form_id: {}", form_id);
                    let mut form_data = form_data_rw.get_untracked().unwrap();
                    form_data.update_with_config(config);
                    form_data_rw.set(Some(form_data));
                    is_loading.set(false);
                }
                Ok(None) => {
                    log!("No data found for form_id: {}", form_id);
                    is_loading.set(false);
                }
                Err(e) => {
                    log!(
                        "Error loading data: {:?} for form_id: {}",
                        e,
                        form_id
                    );
                    load_error.set(Some(e));
                    is_loading.set(false);
                }
            }
        });
    };

    let is_submitting = create_rw_signal(cx, false);
    let submit_error = create_rw_signal(cx, None::<String>);
    let handle_submit = move |ev: SubmitEvent, form_data: Option<FormData>| {
        ev.prevent_default();
        is_submitting.set(true);
        let storage_handler = storage_handler.clone();

        spawn_local(async move {
            if let Some(form_data) = form_data {
                let form_elements = form_data.elements();
                let validation_errors = perform_validation(&form_elements);
                if validation_errors.is_empty() {
                    let result = storage_handler.save_config(&form_data).await;
                    match result {
                        Ok(_) => {
                            log!("Data submitted successfully");
                            is_submitting.set(false);
                        }
                        Err(e) => {
                            log!("Data submission failed: {:?}", e);
                            submit_error.set(Some(e.to_string()));
                        }
                    }
                } else {
                    log!("Form data is invalid");
                    log!("Validation errors: {:?}", validation_errors);
                }
            } else {
                log!("Form data is empty");
            }
            is_submitting.set(false);
        });
    };

    let load_parameters = LoadParameters::new(
        Some(Box::new(handle_load)),
        Some(is_loading),
        Some(load_error),
    );

    let form_button =
        FormButton::new(ButtonType::Save, None).set_enabled(false);
    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(submit_error),
        Some(form_button),
    );

    // Use predefined form elements based on config_type
    let template_name = form_meta.template().unwrap_or("".to_string());
    let profile_name = form_meta.name().unwrap_or("".to_string());

    let app_config = AppConfig::new(template_name, profile_name.clone(), None);
    let form_elements = app_config.form_elements();

    let form_builder = ProfileFormBuilder::new(
        &profile_name,
        form_meta,
        FormType::LoadAndSubmitData(load_parameters, submit_parameters),
    )
    .with_elements(form_elements);

    let form = if is_text_area {
        form_builder.to_text_area().build(cx)
    } else {
        form_builder.build(cx)
    };

    form.to_view()
}
