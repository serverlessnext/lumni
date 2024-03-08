use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::*;
use leptos::logging::log;

use crate::components::buttons::{ButtonType, FormButton, TextLink};
use crate::components::forms::builders::{
    FormType, LoadParameters, ProfileFormBuilder, SubmitParameters,
};
use crate::components::forms::input::*;
use crate::components::forms::{ConfigurationFormMeta, FormData};
use crate::GlobalState;

const ENVIRONMENT_FORM_ID: &str = "EnvironmentForm";

#[component]
pub fn Environment() -> impl IntoView {
    let is_enabled = create_rw_signal(false);

    let form_button =
        FormButton::new(ButtonType::Submit, Some("Set Environment"));
    let on_click = move |event: MouseEvent| {
        event.prevent_default();
        is_enabled.set(!is_enabled.get());
    };

    view! {
        <TextLink
            form_button={form_button}
            enabled=is_enabled.into()
            on_click={on_click}
        />
        // if enabled
        { move || if is_enabled.get() {
            view! {
                <SetEnvironment />
            }.into_view()
        } else {
            view! { "" }.into_view()
         }}
    }
}

#[component]
pub fn SetEnvironment() -> impl IntoView {
    let is_loading = create_rw_signal(false);
    let load_error = create_rw_signal(None::<String>);

    let is_submitting = create_rw_signal(false);
    let submit_error = create_rw_signal(None::<String>);

    let handle_load = {
        let memory_store = use_context::<RwSignal<GlobalState>>()
            .expect("state to have been provided")
            .with(|state| state.store.clone());

        move |form_data_rw: RwSignal<Option<FormData>>| {
            let memory_store = memory_store.clone();
            is_loading.set(true);
            spawn_local(async move {
                let store = memory_store.lock().unwrap();
                match store.load_config(ENVIRONMENT_FORM_ID).await {
                    Ok(Some(config)) => {
                        log!(
                            "Data loaded into environment for form_id: {}",
                            ENVIRONMENT_FORM_ID
                        );
                        let mut form_data =
                            form_data_rw.get_untracked().unwrap();
                        form_data.update_with_config(config);
                        form_data_rw.set(Some(form_data));
                        is_loading.set(false);
                    }
                    Ok(None) => {
                        log!(
                            "No data found for form_id: {}",
                            ENVIRONMENT_FORM_ID
                        );
                        is_loading.set(false);
                    }
                    Err(e) => {
                        log!(
                            "Error loading data: {:?} for form_id: {}",
                            e,
                            ENVIRONMENT_FORM_ID
                        );
                        load_error.set(Some(e));
                        is_loading.set(false);
                    }
                }
            });
        }
    };

    let handle_submit = {
        let memory_store = use_context::<RwSignal<GlobalState>>()
            .expect("state to have been provided")
            .with(|state| state.store.clone());

        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();
            is_submitting.set(true);
            let memory_store = memory_store.clone();

            spawn_local(async move {
                let store = memory_store.lock().unwrap();
                if let Some(form_data) = form_data {
                    let form_elements = form_data.elements();
                    let validation_errors = perform_validation(&form_elements);
                    if validation_errors.is_empty() {
                        let result = store.save_config(&form_data).await;
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
        }
    };

    let load_parameters = LoadParameters::new(Some(Box::new(handle_load)));

    let save_button =
        FormButton::new(ButtonType::Save, None).set_enabled(false);
    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(submit_error),
        Some(save_button),
    );

    let form_meta = ConfigurationFormMeta::with_id(ENVIRONMENT_FORM_ID);
    let form = ProfileFormBuilder::new(
        "Load Form",
        form_meta,
        FormType::LoadAndSubmitData(load_parameters, submit_parameters),
    )
    .to_text_area()
    .build();

    form.to_view()
}
