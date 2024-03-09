use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::logging::{error, log};
use leptos::*;
use leptos_router::use_navigate;
use localencrypt::{LocalEncrypt, LocalStorage, StorageBackend};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;

use crate::components::buttons::{ActionTrigger, ButtonType, FormButton};
use crate::components::forms::builders::{
    build_all, ElementBuilder, InputFieldPattern,
};
use crate::components::forms::input::FormElement;
use crate::components::forms::{
    ConfigurationFormMeta, FormData, FormError, HtmlForm, SubmitFormClassic,
};
use crate::helpers::local_storage::delete_keys_not_matching_prefix;
use crate::vars::{LOCALSTORAGE_PREFIX, ROOT_USERNAME};
use crate::GlobalState;

const PASSWORD_FIELD: &str = "PASSWORD";
const FORM_DATA_MISSING: &str = "form_data does not exist";
const PASSWORD_MISSING: &str = "password does not exist";

#[derive(Clone)]
pub struct AppLogin {
    set_vault: SignalSetter<LocalEncrypt>,
    is_submitting: RwSignal<bool>,
    validation_error: RwSignal<Option<String>>,
    redirect_url: String,
}

impl AppLogin {
    pub fn new(
        set_vault: SignalSetter<LocalEncrypt>,
        redirect_url: String,
    ) -> Self {
        let is_submitting = create_rw_signal(false);
        let validation_error = create_rw_signal(None::<String>);
        Self {
            set_vault,
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
        if let Err(err) = delete_keys_not_matching_prefix(
            format!("{}:", LOCALSTORAGE_PREFIX).as_str(),
        )
        .await
        {
            log!(
                "Error deleting keys: {}",
                err.as_string()
                    .unwrap_or_else(|| String::from("Unknown error"))
            );
        }

        let navigate = use_navigate();
        match StorageBackend::initiate_with_local_storage(
            Some(LOCALSTORAGE_PREFIX),
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

                navigate(&self.redirect_url, Default::default())
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
pub fn LoginForm() -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>()
        .expect("state to have been provided");

    let init_vault =
        create_write_slice(state, |state, vault| state.vault = Some(vault));

    let previous_url = move || {
        create_read_slice(state, |state| {
            state.runtime.as_ref().map(|r| r.previous_url().clone())
        })
    };

    let redirect_url = previous_url().get_untracked().unwrap_or_default();

    // Page is in a loading state until we know if the user is defined
    let is_user_defined = create_rw_signal(false);
    let is_loading = create_rw_signal(true);
    spawn_local({
        async move {
            let user_exists =
                LocalStorage::user_exists(LOCALSTORAGE_PREFIX, ROOT_USERNAME)
                    .await;
            is_user_defined.set(user_exists);
            is_loading.set(false);
        }
    });

    let app_login = AppLogin::new(init_vault, redirect_url);

    view! {
        { move ||
            if is_loading.get() {
                view! {
                    <div>"Loading..."</div>
                }.into_view()
            } else if is_user_defined.get() {
                let app_login = app_login.clone();
                let validation_error = app_login.validation_error();
                if app_login.validation_error.get().is_some() {
                    view! {
                        <LoginUser app_login/>
                        { reset_password_view(is_user_defined, validation_error) }
                    }.into_view()
                } else {
                    view! {
                        <LoginUser app_login/>
                    }.into_view()
                }
            } else {
                view! {
                    <CreateUser app_login=app_login.clone()/>
                }.into_view()
            }
        }
    }
}

#[component]
pub fn LoginUser(app_login: AppLogin) -> impl IntoView {
    let is_submitting = app_login.is_submitting();
    let validation_error = app_login.validation_error();
    let elements: Vec<FormElement> =
        build_all(vec![ElementBuilder::with_pattern(
            InputFieldPattern::PasswordCheck,
        )]);

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let form_login = HtmlForm::new("Login", form_meta, None, elements);

    let handle_form_submission =
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            app_login.set_password_and_submit(ev, form_data);
        };

    let login_button = FormButton::new(ButtonType::Login, None);
    let login_form = SubmitFormClassic::new(
        form_login,
        Box::new(handle_form_submission),
        is_submitting,
        validation_error,
        Some(login_button),
    );

    login_form.to_view()
}

#[component]
pub fn CreateUser(app_login: AppLogin) -> impl IntoView {
    let is_submitting = app_login.is_submitting();
    let validation_error = app_login.validation_error();
    let elements: Vec<FormElement> =
        build_all(vec![ElementBuilder::with_pattern(
            InputFieldPattern::PasswordCheck,
        )]);

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let form_create =
        HtmlForm::new("Create Password", form_meta, None, elements);

    let handle_form_submission =
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            app_login.set_password_and_submit(ev, form_data);
        };

    let create_button = FormButton::new(ButtonType::Create, None);
    let create_form = SubmitFormClassic::new(
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
        .and_then(|form_data| {
            form_data
                .export_config()
                .get(PASSWORD_FIELD)
                .cloned()
                .ok_or_else(|| FormError::ValidationError {
                    field: PASSWORD_FIELD.to_string(),
                    details: PASSWORD_MISSING.to_string(),
                })
        })
}

#[component]
pub fn LoginFormDebug() -> impl IntoView {
    debug_login();
    view! {
        <div>
            <p>"Debug logged in..."</p>
        </div>
    }
}

fn debug_login() {
    // generate both unique user and password for each session
    // in the event confidential data is stored during development
    // its at least encrypted with a unique password
    let debug_environment = "DEBUG";
    let debug_username = format!("debug-user-{}", Uuid::new_v4());
    let debug_password = format!("debug-password-{}", Uuid::new_v4());

    let state = use_context::<RwSignal<GlobalState>>()
        .expect("state to have been provided");

    // Create writable state slices for the vault and initialization status.
    let set_vault =
        create_write_slice(state, |state, vault| state.vault = Some(vault));

    spawn_local(async move {
        if let Err(err) = delete_keys_not_matching_prefix(
            format!("{}:", LOCALSTORAGE_PREFIX).as_str(),
        )
        .await
        {
            log!(
                "Error deleting keys: {}",
                err.as_string()
                    .unwrap_or_else(|| String::from("Unknown error"))
            );
        }

        match StorageBackend::initiate_with_local_storage(
            Some(debug_environment),
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
                let navigate = use_navigate();
                navigate("/", Default::default());
            }
            Err(err) => {
                let msg = err.to_string();
                error!("{}", msg);
            }
        }
    });
}

async fn reset_vault_action() -> Result<(), FormError> {
    let storage_backend = StorageBackend::initiate_with_local_storage(
        Some(LOCALSTORAGE_PREFIX),
        ROOT_USERNAME,
        None,
    )
    .await;
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
    is_user_defined: RwSignal<bool>,
    validation_error: RwSignal<Option<String>>,
) -> View {
    let action = Arc::new(move || async move {
        match reset_vault_action().await {
            Ok(_) => {
                validation_error.set(None);
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
        <div class="bg-white rounded shadow p-4">
        <div class="flex flex-col mb-4">
            <p class="text-gray-700 text-lg">
                "You have the option to reset the password. Please be aware, the configuration database is encrypted and can't be restored without the correct password. If you choose to proceed with the password reset, the database stored in this application will be permanently erased. This irreversible action should be carefully considered."
            </p>
        </div>
        {reset_button.render_view()}
        </div>
    }.into_view()
}
