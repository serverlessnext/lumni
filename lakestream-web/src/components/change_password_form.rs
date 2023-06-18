use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::*;
use localencrypt::StorageBackend;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::components::buttons::ButtonType;
use crate::components::form_input::{
    FormElement, InputFieldBuilder, InputFieldPattern,
};
use crate::components::forms::SingleInputForm;

const ROOT_USERNAME: &str = "admin";

#[component]
pub fn ChangePasswordForm(cx: Scope) -> impl IntoView {
    let password_ref: NodeRef<Input> = create_node_ref(cx);
    let new_password_ref: NodeRef<Input> = create_node_ref(cx);

    let (is_old_password_valid, set_is_old_password_valid) =
        create_signal(cx, false);

    let is_old_password_not_valid =
        (move || !is_old_password_valid.get()).derive_signal(cx);
    let error_signal = create_rw_signal(cx, None);

    let handle_old_password_submission = move |ev: SubmitEvent, _: bool| {
        ev.prevent_default();
        let password = password_ref().expect("password to exist").value();

        spawn_local(async move {
            let storage_backend = StorageBackend::initiate_with_local_storage(
                ROOT_USERNAME,
                Some(&password),
            )
            .await;
            match storage_backend {
                Ok(backend) => match backend.validate_password().await {
                    Ok(valid) => {
                        if valid {
                            set_is_old_password_valid.set(true);
                        } else {
                            error_signal.set(Some(
                                "Invalid password. Please try again."
                                    .to_string(),
                            ));
                        }
                    }
                    Err(err) => error_signal
                        .set(Some(format!("Error: {}", err.to_string()))),
                },
                Err(err) => error_signal
                    .set(Some(format!("Error: {}", err.to_string()))),
            }
        });
    };

    let handle_new_password_submission = move |ev: SubmitEvent, _: bool| {
        ev.prevent_default();
        let password = password_ref().expect("password to exist").value();
        let new_password =
            new_password_ref().expect("new password to exist").value();

        spawn_local(async move {
            let storage_backend = StorageBackend::initiate_with_local_storage(
                ROOT_USERNAME,
                Some(&password),
            )
            .await;
            match storage_backend {
                Ok(backend) => {
                    match backend
                        .change_password(&password, &new_password)
                        .await
                    {
                        Ok(_) => log!("Password changed successfully"),
                        Err(err) => {
                            let msg = err.to_string();
                            web_sys::console::log_1(&JsValue::from_str(&msg));
                            error_signal.set(Some(msg));
                        }
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

    let handle_old_password_submission =
        Arc::new(handle_old_password_submission);
    let handle_new_password_submission =
        Arc::new(handle_new_password_submission);

    let form_config_old_password = SingleInputForm::new(
        handle_old_password_submission.clone(),
        false,
        ButtonType::Login(Some("Validate Current Password".to_string())),
        match InputFieldBuilder::with_pattern(InputFieldPattern::PasswordCheck)
            .build()
        {
            FormElement::InputField(field_data) => field_data,
        },
    );

    let form_config_new_password = SingleInputForm::new(
        handle_new_password_submission,
        false,
        ButtonType::Change(Some("Change password".to_string())),
        match InputFieldBuilder::with_pattern(InputFieldPattern::PasswordChange)
            .build()
        {
            FormElement::InputField(field_data) => field_data,
        },
    );

    view! {
        cx,
        <div class="px-2 py-2">
            {form_config_old_password.render_view(cx, password_ref.clone(), is_old_password_not_valid)}
            {form_config_new_password.render_view(cx, new_password_ref.clone(), is_old_password_valid.into())}
            {move || if error_signal.get().is_some() {
                view! {
                    cx,
                    <div class="text-red-500">
                        { error_signal.get().unwrap_or("".to_string()) }
                    </div>
                }
            } else {
                view! { cx, <div></div> }
            }}
        </div>
    }
}
