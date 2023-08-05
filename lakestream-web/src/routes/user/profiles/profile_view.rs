use leptos::ev::SubmitEvent;
use leptos::*;

use super::{Profile, ProfileList};
use super::templates::{ConfigTemplate, Environment, ObjectStoreS3};
use crate::builders::{
    ProfileFormBuilder, FormType, LoadParameters, SubmitParameters,
};
use crate::components::form_input::perform_validation;
use crate::components::forms::{
    ConfigurationFormMeta, FormData, FormStorageHandler,
};

#[component]
pub fn ProfileView(
    cx: Scope,
    storage_handler: FormStorageHandler,
    form_meta: ConfigurationFormMeta,
) -> impl IntoView {
    let is_loading = create_rw_signal(cx, false);
    let load_error = create_rw_signal(cx, None::<String>);

    let form_id_clone = form_meta.form_id.clone();
    let storage_handler_clone = storage_handler.clone();

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
                let form_state = form_data.form_state().to_owned();
                let validation_errors = perform_validation(&form_state);
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

    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(submit_error),
        None,
    );

    // Use predefined form elements based on config_type
    let template_name = form_meta.template_name;
    let config_name = form_meta.config_name;
    let form_elements = match template_name.as_str() {
        "ObjectStoreS3" => {
            Profile::ObjectStoreS3(ObjectStoreS3::new(&config_name))
        }
        _ => Profile::Environment(Environment::new(&config_name)),
    }
    .form_elements(&template_name);

    let form_tags = form_meta.tags;

    let form_id = form_meta.form_id;
    let form = ProfileFormBuilder::new(
        &config_name,
        &form_id,
        form_tags,
        FormType::LoadAndSubmitData(load_parameters, submit_parameters),
    )
    .with_elements(form_elements)
    .build(cx);

    form.to_view()
}
