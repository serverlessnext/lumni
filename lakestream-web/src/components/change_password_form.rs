use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::StorageBackend;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;

use crate::builders::{build_all, InputFieldPattern, TextBoxBuilder};
use crate::components::buttons::{ButtonType, FormButton};
use crate::components::form_input::FormElement;
use crate::components::forms::{
    FormData, FormError, HtmlForm, SubmitFormClassic,
};
use crate::vars::{LOCALSTORAGE_PREFIX, ROOT_USERNAME};

const INTERNAL_ERROR: &str = "An internal error occurred: ";
const INVALID_PASSWORD: &str = "Invalid password. Please try again.";
const FORM_DATA_MISSING: &str = "form_data does not exist";
const PASSWORD_FIELD: &str = "PASSWORD";
const PASSWORD_MISSING: &str = "password does not exist";

#[component]
pub fn ChangePasswordForm(cx: Scope) -> impl IntoView {
    let password_validated = create_rw_signal(cx, None::<String>);
    let is_submitting = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    let elements_validation: Vec<FormElement> =
        build_all(vec![TextBoxBuilder::with_pattern(
            InputFieldPattern::PasswordCheck,
        )]);
    let elements_change: Vec<FormElement> =
        build_all(vec![TextBoxBuilder::with_pattern(
            InputFieldPattern::PasswordChange,
        )]);

    let form_validation = HtmlForm::new(
        cx,
        "Validate Password",
        &Uuid::new_v4().to_string(),
        None,
        elements_validation,
    );
    let form_change = HtmlForm::new(
        cx,
        "Change Password",
        &Uuid::new_v4().to_string(),
        None,
        elements_change,
    );

    let handle_password_validation = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();

            let password_from_user = match handle_internal_error(
                extract_password(form_data),
                validation_error,
            ) {
                Some(password) => password,
                None => {
                    is_submitting.set(false);
                    return;
                }
            };

            spawn_local(async move {
                let backend_result = initiate_storage_backend(
                    ROOT_USERNAME,
                    &password_from_user,
                )
                .await;
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
                    validation_error.set(Some(format!(
                        "{}{}",
                        INTERNAL_ERROR, "password does not exist"
                    )));
                    is_submitting.set(false);
                    return;
                }
            };

            let new_password = match handle_internal_error(
                extract_password(form_data),
                validation_error,
            ) {
                Some(password) => password,
                None => {
                    is_submitting.set(false);
                    return;
                }
            };

            spawn_local(async move {
                let backend_result =
                    initiate_storage_backend(ROOT_USERNAME, &password).await;
                if let Some(backend) =
                    handle_internal_error(backend_result, validation_error)
                {
                    let password_change_result = backend
                        .change_password(
                            Some(LOCALSTORAGE_PREFIX),
                            &password,
                            &new_password,
                        )
                        .await
                        .map_err(FormError::LocalEncryptError);
                    if handle_internal_error(
                        password_change_result,
                        validation_error,
                    )
                    .is_some()
                    {
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
    let login_button =
        FormButton::new(ButtonType::Login, Some("Validate Current Password"));
    let validation_form = SubmitFormClassic::new(
        cx,
        form_validation,
        Box::new(handle_password_validation),
        is_submitting,
        validation_error,
        Some(login_button),
    );

    let change_button =
        FormButton::new(ButtonType::Change, Some("Change Password"));
    let change_form = SubmitFormClassic::new(
        cx,
        form_change,
        Box::new(handle_password_change),
        is_submitting,
        validation_error,
        Some(change_button),
    );

    view! { cx,
        { move ||
            if password_validated.get().is_none() {
                validation_form.to_view()
            } else {
                change_form.to_view()
            }
        }
    }
}

async fn initiate_storage_backend(
    username: &str,
    password: &str,
) -> Result<StorageBackend, FormError> {
    StorageBackend::initiate_with_local_storage(
        Some(LOCALSTORAGE_PREFIX),
        username,
        Some(password),
    )
    .await
    .map_err(FormError::from)
}

fn extract_password(form_data: Option<FormData>) -> Result<String, FormError> {
    form_data
        .ok_or_else(|| FormError::SubmitError(FORM_DATA_MISSING.to_string()))
        .and_then(|data| {
            data.to_hash_map()
                .get(PASSWORD_FIELD)
                .cloned()
                .ok_or_else(|| FormError::ValidationError {
                    field: PASSWORD_FIELD.to_string(),
                    details: PASSWORD_MISSING.to_string(),
                })
        })
}

fn handle_internal_error<T>(
    result: Result<T, FormError>,
    validation_error: RwSignal<Option<String>>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            error!("{}", err); // log error to console

            let error_message = match &err {
                FormError::SubmitError(msg) => {
                    format!("{}{}", INTERNAL_ERROR, msg)
                }
                FormError::ValidationError { field, details } => {
                    format!(
                        "{}Validation error in field '{}': {}",
                        INTERNAL_ERROR, field, details
                    )
                }
                FormError::LocalEncryptError(err) => {
                    format!("{}{}", INTERNAL_ERROR, err.to_string())
                }
            };

            validation_error.set(Some(error_message));
            None
        }
    }
}
