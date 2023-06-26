use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::*;
use leptos_router::use_navigate;
use localencrypt::{LocalEncrypt, LocalStorage, StorageBackend};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;

use crate::builders::{build_all, InputFieldPattern, TextBoxBuilder};
use crate::components::buttons::{ActionTrigger, ButtonType, FormButton};
use crate::components::form_input::FormElement;
use crate::components::forms::{
    FormData, FormError, HtmlFormMeta, SubmitFormClassic,
};
use crate::GlobalState;

const ROOT_USERNAME: &str = "admin";
const PASSWORD_FIELD: &str = "PASSWORD";
const FORM_DATA_MISSING: &str = "form_data does not exist";
const PASSWORD_MISSING: &str = "password does not exist";

#[derive(Clone)]
pub struct AppLogin {
    cx: Scope,
    set_vault: SignalSetter<LocalEncrypt>,
    set_vault_initialized: SignalSetter<bool>,
    is_submitting: RwSignal<bool>,
    validation_error: RwSignal<Option<String>>,
    redirect_url: String,
}

impl AppLogin {
    pub fn new(
        cx: Scope,
        set_vault: SignalSetter<LocalEncrypt>,
        set_vault_initialized: SignalSetter<bool>,
        redirect_url: String,
    ) -> Self {
        let is_submitting = create_rw_signal(cx, false);
        let validation_error = create_rw_signal(cx, None::<String>);
        Self {
            cx,
            set_vault,
            set_vault_initialized,
            is_submitting,
            validation_error,
            redirect_url,
        }
    }

    pub fn is_submitting(&self) -> RwSignal<bool> {
        self.is_submitting
    }

    pub fn validation_error(&self) -> RwSignal<Option<String>> {
        self.validation_error
    }

    pub async fn initialize_and_navigate(
        &self,
        password: String,
        is_submitting: RwSignal<bool>,
        validation_error: RwSignal<Option<String>>,
    ) {
        let navigate = use_navigate(self.cx);
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

                self.set_vault.set(local_encrypt);
                self.set_vault_initialized.set(true);

                if let Err(e) = navigate(&self.redirect_url, Default::default())
                {
                    error!("Error navigating to {}: {}", &self.redirect_url, e);
                }
            }
            Err(err) => {
                let msg = err.to_string();
                error!("Error initializing vault: {}", msg);
                validation_error.set(Some(msg));
                is_submitting.set(false);
            }
        }
    }

    pub fn set_password_and_submit(
        &self,
        ev: SubmitEvent,
        form_data: Option<FormData>,
    ) {
        ev.prevent_default();
        let password = match extract_password(form_data) {
            Ok(password) => password,
            Err(err) => {
                self.validation_error.set(Some(err.to_string()));
                return;
            }
        };
        let app_login = self.clone();
        spawn_local(async move {
            let is_submitting = app_login.is_submitting();
            let validation_error = app_login.validation_error();
            app_login
                .initialize_and_navigate(
                    password,
                    is_submitting,
                    validation_error,
                )
                .await;
        });
    }
}

#[component]
pub fn LoginForm(cx: Scope) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    let init_vault =
        create_write_slice(cx, state, |state, vault| state.vault = Some(vault));
    let vault_initialized =
        create_write_slice(cx, state, |state, initialized| {
            if let Some(runtime) = &mut state.runtime {
                runtime.set_vault_initialized(initialized);
            }
        });
    let previous_url = create_read_slice(cx, state, |state| {
        state.runtime.as_ref().map(|r| r.previous_url().clone())
    });

    let redirect_url = previous_url().unwrap_or_default();

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

    let app_login =
        AppLogin::new(cx, init_vault, vault_initialized, redirect_url);

    view! { cx,
        { move ||
            if is_loading.get() {
                view! {
                    cx,
                    <div>"Loading..."</div>
                }.into_view(cx)
            } else if is_user_defined.get() {
                view! {
                    cx,
                    <LoginUser app_login=app_login.clone()/>
                }.into_view(cx)
            } else {
                view! {
                    cx,
                    <CreateUser app_login=app_login.clone()/>
                }.into_view(cx)
            }
        }
    }
}

#[component]
pub fn LoginUser(cx: Scope, app_login: AppLogin) -> impl IntoView {
    let is_submitting = app_login.is_submitting();
    let validation_error = app_login.validation_error();
    let elements: Vec<FormElement> =
        build_all(vec![TextBoxBuilder::with_pattern(
            InputFieldPattern::PasswordCheck,
        )]);

    let form_login =
        HtmlFormMeta::new("Login", &Uuid::new_v4().to_string(), elements);

    let handle_form_submission =
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            app_login.set_password_and_submit(ev, form_data);
        };

    let login_button = FormButton::new(ButtonType::Login, None);
    let login_form = SubmitFormClassic::new(
        cx,
        form_login,
        Box::new(handle_form_submission),
        is_submitting,
        validation_error,
        Some(login_button),
    );

    login_form.to_view()
}

#[component]
pub fn CreateUser(cx: Scope, app_login: AppLogin) -> impl IntoView {
    let is_submitting = app_login.is_submitting();
    let validation_error = app_login.validation_error();
    let elements: Vec<FormElement> =
        build_all(vec![TextBoxBuilder::with_pattern(
            InputFieldPattern::PasswordCheck,
        )]);

    let form_create = HtmlFormMeta::new(
        "Create Password",
        &Uuid::new_v4().to_string(),
        elements,
    );

    let handle_form_submission =
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            app_login.set_password_and_submit(ev, form_data);
        };

    let create_button = FormButton::new(ButtonType::Create, None);
    let create_form = SubmitFormClassic::new(
        cx,
        form_create,
        Box::new(handle_form_submission),
        is_submitting,
        validation_error,
        Some(create_button),
    );

    create_form.to_view()
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
    let debug_username = format!("debug-user-{}", Uuid::new_v4());
    let debug_password = format!("debug-password-{}", Uuid::new_v4());

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
                if let Err(e) = navigate("/", Default::default()) {
                    log!("Error navigating to {}: {}", "/", e);
                }
            }
            Err(err) => {
                let msg = err.to_string();
                error!("{}", msg);
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
    let action = Arc::new(move || async move {
        match reset_vault_action().await {
            Ok(_) => {
                is_user_defined.set(false);
                Ok(())
            }
            Err(err) => Err(err),
        }
    });

    let reset_button = ActionTrigger::new(
        FormButton::new(ButtonType::Reset, Some("Reset Password")),
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
