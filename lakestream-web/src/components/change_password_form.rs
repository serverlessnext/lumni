
use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::StorageBackend;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;

use crate::components::buttons::ButtonType;
use crate::components::form_input::{
    build_all, FormElement, InputFieldBuilder, InputFieldPattern,
};
use crate::components::forms::{
    CustomFormHandler, FormData, HtmlForm, FormError,
};

const ROOT_USERNAME: &str = "admin";
const PASSWORD_FIELD: &str = "PASSWORD";
const INTERNAL_ERROR: &str = "An internal error occurred: ";
const INVALID_PASSWORD: &str = "Invalid password. Please try again.";
const FORM_DATA_MISSING: &str = "form_data does not exist";
const PASSWORD_MISSING: &str = "password does not exist";


#[component]
pub fn ChangePasswordForm(cx: Scope) -> impl IntoView {
    let password_validated = create_rw_signal(cx, None::<String>);
    let is_submitting = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    let elements_validation: Vec<FormElement> = build_all(vec![InputFieldBuilder::with_pattern(InputFieldPattern::PasswordCheck)]);
    let elements_change: Vec<FormElement> = build_all(vec![InputFieldBuilder::with_pattern(InputFieldPattern::PasswordChange)]);

    let form_validation = HtmlForm::new("Validate Password", &Uuid::new_v4().to_string(), elements_validation);
    let form_change = HtmlForm::new("Change Password", &Uuid::new_v4().to_string(), elements_change);

    let handle_password_validation = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();

            let password_from_user = match handle_internal_error(extract_password(form_data), validation_error) {
                Some(password) => password,
                None => {
                    is_submitting.set(false);
                    return;
                }
            };

            spawn_local(async move {
                let backend_result = initiate_storage_backend(ROOT_USERNAME, &password_from_user).await;
                if backend_result.is_ok() {
                    log!("Password validated successfully");
                    validation_error.set(None);
                    password_validated.set(Some(password_from_user));
                } else {
                    error!("{}", backend_result.unwrap_err()); // log error to console
                    validation_error.set(Some(INVALID_PASSWORD.to_string()));
                }
                is_submitting.set(false);
            });
        }
    };

    let handle_password_change = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();

            let password = match password_validated.get() {
                Some(password) => password,
                None => {
                    validation_error.set(Some(format!("{}{}", INTERNAL_ERROR, "password does not exist")));
                    is_submitting.set(false);
                    return;
                }
            };

            let new_password = match handle_internal_error(extract_password(form_data), validation_error) {
                Some(password) => password,
                None => {
                    is_submitting.set(false);
                    return;
                }
            };

            spawn_local(async move {
                let backend_result = initiate_storage_backend(ROOT_USERNAME, &password).await;
                if let Some(backend) = handle_internal_error(backend_result, validation_error) {
                    let password_change_result = backend
                        .change_password(&password, &new_password)
                        .await
                        .map_err(FormError::LocalEncryptError);
                    if handle_internal_error(password_change_result, validation_error).is_some() {
                        log!("Password changed successfully");
                        validation_error.set(None);
                        password_validated.set(Some(new_password));
                    }
                }
                is_submitting.set(false);
            });
        }
    };

    // Create a custom form handlers with the defined functions
    let validation_form_handler = CustomFormHandler::new(
        cx,
        form_validation,
        Box::new(handle_password_validation),
        is_submitting,
        validation_error,
        Some(ButtonType::Login(Some("Validate Current Password".to_string()))),
    );

    let change_form_handler = CustomFormHandler::new(
        cx,
        form_change,
        Box::new(handle_password_change),
        is_submitting,
        validation_error,
        Some(ButtonType::Change(Some("Change Password".to_string()))),
    );

    view! { cx,
        { move ||
            if password_validated.get().is_none() {
                validation_form_handler.create_view()
            } else {
                change_form_handler.create_view()
            }
        }
    }
}

async fn initiate_storage_backend(username: &str, password: &str) -> Result<StorageBackend, FormError> {
    StorageBackend::initiate_with_local_storage(username, Some(password)).await.map_err(FormError::from)
}

fn extract_password(form_data: Option<FormData>) -> Result<String, FormError> {
    form_data
        .ok_or_else(|| FormError::SubmitError(FORM_DATA_MISSING.to_string()))
        .and_then(|data| {
            data.to_hash_map()
                .get(PASSWORD_FIELD)
                .cloned()
                .ok_or_else(|| FormError::ValidationError { field: PASSWORD_FIELD.to_string(), details: PASSWORD_MISSING.to_string() })
        })
}

fn handle_internal_error<T>(result: Result<T, FormError>, validation_error: RwSignal<Option<String>>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            error!("{}", err); // log error to console

            let error_message = match &err {
                FormError::SubmitError(msg) => format!("{}{}", INTERNAL_ERROR, msg),
                FormError::ValidationError { field, details } => {
                    format!("{}Validation error in field '{}': {}", INTERNAL_ERROR, field, details)
                },
                FormError::LocalEncryptError(err) => format!("{}{}", INTERNAL_ERROR, err.to_string()),
            };

            validation_error.set(Some(error_message));
            None
        }
    }
}

