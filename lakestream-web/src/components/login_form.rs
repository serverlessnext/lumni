use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::*;
use leptos_router::use_navigate;
use localencrypt::{LocalEncrypt, LocalStorage, StorageBackend};
use uuid::Uuid;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::components::buttons::{ActionTrigger, ButtonType};
use crate::components::form_input::{
    build_all, FormElement, InputFieldBuilder, InputFieldPattern,
};
use crate::components::forms::{
    CustomFormHandler, FormData, FormError, HtmlForm,
};
use crate::GlobalState;

const ROOT_USERNAME: &str = "admin";
const PASSWORD_FIELD: &str = "PASSWORD";
const FORM_DATA_MISSING: &str = "form_data does not exist";
const PASSWORD_MISSING: &str = "password does not exist";

#[component]
pub fn LoginForm(cx: Scope) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    // Page is in a loading state until we know if the user is defined
    let is_user_defined = create_rw_signal(cx, false);
    let is_loading = create_rw_signal(cx, true);
    spawn_local({
        async move {
            let user_exists = LocalStorage::user_exists(ROOT_USERNAME).await;
            is_user_defined.set(user_exists);
            is_loading.set(false);
        }
    });

    // Create writable state slices for the vault and initialization status.
    let set_vault =
        create_write_slice(cx, state, |state, vault| state.vault = Some(vault));
    let set_vault_initialized =
        create_write_slice(cx, state, |state, initialized| {
            if let Some(runtime) = &mut state.runtime {
                runtime.set_vault_initialized(initialized);
            }
        });

    // Create a readable state slice for the previous URL.
    let previous_url = create_read_slice(cx, state, |state| {
        state.runtime.as_ref().map(|r| r.previous_url().clone())
    });
    let redirect_url = previous_url().unwrap_or_default();

    let is_submitting = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    let elements: Vec<FormElement> =
        build_all(vec![InputFieldBuilder::with_pattern(
            InputFieldPattern::PasswordCheck,
        )]);

    let form_login =
        HtmlForm::new("Login", &Uuid::new_v4().to_string(), elements.clone());
    let form_create =
        HtmlForm::new("Create Password", &Uuid::new_v4().to_string(), elements);

    // form submission is used for both login of existing user and new users
    // if user does not exist it will be created when initiating StorageBackend
    let handle_form_submission = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();

            let password = match extract_password(form_data) {
                Ok(password) => password,
                Err(err) => {
                    validation_error.set(Some(err.to_string()));
                    return;
                }
            };

            let navigate = use_navigate(cx);
            let redirect_url = redirect_url.clone();

            spawn_local(async move {
                match StorageBackend::initiate_with_local_storage(
                    ROOT_USERNAME,
                    Some(&password),
                )
                .await
                {
                    Ok(storage_backend) => {
                        let local_encrypt = LocalEncrypt::builder()
                            .with_backend(storage_backend)
                            .build();

                        set_vault.set(local_encrypt);
                        set_vault_initialized.set(true);

                        if let Err(e) =
                            navigate(&redirect_url, Default::default())
                        {
                            error!(
                                "Error navigating to {}: {}",
                                &redirect_url, e
                            );
                        }
                    }
                    Err(err) => {
                        let msg = err.to_string();
                        web_sys::console::log_1(&JsValue::from_str(&msg));
                        validation_error.set(Some(msg));
                        is_submitting.set(false);
                    }
                }
            });
        }
    };

    // Existing user
    let login_form_handler = CustomFormHandler::new(
        cx,
        form_login,
        Box::new(handle_form_submission.clone()),
        is_submitting,
        validation_error,
        Some(ButtonType::Login(None)),
    );

    // New user gets this view
    let create_form_handler = CustomFormHandler::new(
        cx,
        form_create,
        Box::new(handle_form_submission),
        is_submitting,
        validation_error,
        Some(ButtonType::Create(Some("Create new password".to_string()))),
    );

    view! { cx,
        { move ||
            if is_loading.get() {
                view! {
                    cx,
                    <div>"Loading..."</div>
                }.into_view(cx)
            } else if is_user_defined.get() {
                login_form_handler.create_view()
            } else {
                create_form_handler.create_view()
            }
        }
    }
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

#[component]
pub fn LoginFormDebug(cx: Scope) -> impl IntoView {
    debug_login(cx);
    view! {
        cx,
        <div>
            <p>"Debug login"</p>
        </div>
    }
}

fn debug_login(cx: Scope) {
    // generate both unique user and password for each session
    // in the event confidential data is stored during development
    // its at least encrypted with a unique password
    let debug_username = format!("debug-user-{}", Uuid::new_v4()).to_string();
    let debug_password =
        format!("debug-password-{}", Uuid::new_v4()).to_string();

    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    // Create writable state slices for the vault and initialization status.
    let set_vault =
        create_write_slice(cx, state, |state, vault| state.vault = Some(vault));
    let set_vault_initialized =
        create_write_slice(cx, state, |state, initialized| {
            if let Some(runtime) = &mut state.runtime {
                runtime.set_vault_initialized(initialized);
            }
        });

    spawn_local(async move {
        match StorageBackend::initiate_with_local_storage(
            &debug_username,
            Some(&debug_password),
        )
        .await
        {
            Ok(storage_backend) => {
                let local_encrypt = LocalEncrypt::builder()
                    .with_backend(storage_backend)
                    .build();

                set_vault(local_encrypt);
                set_vault_initialized(true);
                let navigate = use_navigate(cx);
                if let Err(e) = navigate(&"/", Default::default()) {
                    log!("Error navigating to {}: {}", "/", e);
                }
            }
            Err(err) => {
                let msg = err.to_string();
                web_sys::console::log_1(&JsValue::from_str(&msg));
            }
        }
    });
}

async fn reset_vault_action() -> Result<(), FormError> {
    let storage_backend =
        StorageBackend::initiate_with_local_storage(ROOT_USERNAME, None).await;
    match storage_backend {
        Ok(backend) => match backend.hard_reset().await {
            Ok(_) => {
                log!("Vault reset successfully");
                Ok(())
            }
            Err(err) => {
                log!("Error resetting vault: {:?}", err);
                Err(FormError::SubmitError(format!(
                    "Error resetting vault: {:?}",
                    err
                )))
            }
        },
        Err(err) => {
            log!("Error creating storage backend: {:?}", err);
            Err(FormError::SubmitError(format!(
                "Error creating storage backend: {:?}",
                err
            )))
        }
    }
}

fn reset_password_view(
    cx: Scope,
    is_user_defined: RwSignal<bool>,
    error_signal: RwSignal<Option<String>>,
) -> View {
    let action = Arc::new(move || {
        let is_user_defined = is_user_defined.clone();
        async move {
            match reset_vault_action().await {
                Ok(_) => {
                    is_user_defined.set(false);
                    Ok(())
                }
                Err(err) => Err(err),
            }
        }
    });

    let reset_button = ActionTrigger::new(
        ButtonType::Reset(Some("Reset Password".to_string())),
        action,
    );
    view ! {
        cx,
        <div class="bg-white rounded shadow p-4">
        <div class="flex flex-col mb-4">
            <div class="text-red-500">
                { move || error_signal.get().unwrap_or("".to_string()) }
            </div>
            <p class="text-gray-700 text-lg">
                "You have the option to reset the password. Please be aware, the configuration database is encrypted and can't be restored without the correct password. If you choose to proceed with the password reset, the database stored in this application will be permanently erased. This irreversible action should be carefully considered."
            </p>
        </div>
        {reset_button.render_view(cx)}
        </div>
    }.into_view(cx)
}
