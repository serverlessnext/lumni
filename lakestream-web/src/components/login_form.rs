use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::*;
use leptos_router::use_navigate;
use localencrypt::{LocalEncrypt, LocalStorage, StorageBackend};
use uuid::Uuid;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::components::buttons::{ActionTrigger, ButtonType};
use crate::components::form_input::{FormFieldBuilder, InputFieldPattern, FormElement};
use crate::components::forms::{FormError, SingleInputForm};
use crate::GlobalState;

const ROOT_USERNAME: &str = "admin";

#[component]
pub fn LoginForm(cx: Scope) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    // Page must start in a loading state
    let is_loading = create_rw_signal(cx, true);

    // Create writable state slices for the vault and initialization status.
    let set_vault =
        create_write_slice(cx, state, |state, vault| state.vault = Some(vault));
    let set_vault_initialized =
        create_write_slice(cx, state, |state, initialized| {
            if let Some(runtime) = &mut state.runtime {
                runtime.set_vault_initialized(initialized);
            }
        });

    // assume user is not defined as default
    let is_user_defined = create_rw_signal(cx, false);
    let is_user_undefined = (move || !is_user_defined.get()).derive_signal(cx);

    // Create an error message signal
    let error_signal = create_rw_signal(cx, None);

    // Create a readable state slice for the previous URL.
    let previous_url = create_read_slice(cx, state, |state| {
        state.runtime.as_ref().map(|r| r.previous_url().clone())
    });
    // Default to the home page if no previous URL is set.
    let redirect_url = previous_url().unwrap_or_default();
    let password_ref: NodeRef<Input> = create_node_ref(cx);

    let handle_submission = move |ev: SubmitEvent, _: bool| {
        ev.prevent_default();
        let password = password_ref().expect("password to exist").value();
        // clone so original does not get moved into the closure
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

                    set_vault(local_encrypt);
                    set_vault_initialized(true);
                    let navigate = use_navigate(cx);
                    if let Err(e) = navigate(&redirect_url, Default::default())
                    {
                        log!("Error navigating to {}: {}", &redirect_url, e);
                    }
                }
                Err(err) => {
                    let msg = err.to_string();
                    web_sys::console::log_1(&JsValue::from_str(&msg));
                    error_signal.set(Some(msg));
                }
            }
        });
    };

    // check if user is defined
    create_effect(cx, move |_| {
        spawn_local({
            async move {
                let user_exists =
                    LocalStorage::user_exists(ROOT_USERNAME).await;
                is_user_defined.set(user_exists);
                is_loading.set(false);
            }
        });
    });

    let handle_submission = Arc::new(handle_submission);

    let form_config_user_defined = SingleInputForm::new(
        handle_submission.clone(),
        true,
        ButtonType::Login(None),
        match FormFieldBuilder::with_pattern(InputFieldPattern::PasswordCheck) {
            FormElement::InputField(field_data) => field_data,
        },
    );

    let form_config_user_undefined = SingleInputForm::new(
        handle_submission,
        false,
        ButtonType::Create(Some("Create new password".to_string())),
        match FormFieldBuilder::with_pattern(InputFieldPattern::PasswordCheck) {
            FormElement::InputField(field_data) => field_data,
        },
    );

    view! {
        cx,
        {move || if is_user_defined.get() {
            view! {
                cx,
                <div class="px-2 py-2">
                {form_config_user_defined.render_view(cx, password_ref.clone(), is_user_defined.into())}
                {move || if error_signal.get().is_some() {
                    view! {
                        cx,
                        <div>
                            {reset_password_view(cx, is_user_defined.clone(), error_signal)}
                        </div>
                    }
                } else {
                    view! { cx, <div></div> }
                }}
                </div>
            }
        } else if is_loading.get() {
            view! {
                cx,
                <div>"Loading..."</div>
            }
        } else {
            view! {
                cx,
                <div class="px-2 py-2">
                {form_config_user_undefined.render_view(cx, password_ref.clone(), is_user_undefined.into())}
                </div>
            }
        }}
    }
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
