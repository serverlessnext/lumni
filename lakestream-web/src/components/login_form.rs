
use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::html::Input;
use leptos::*;
use leptos_router::use_navigate;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::components::buttons::{FormSubmitButton, SubmitButtonType};
use crate::stringvault::StringVault;
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

    // Create an error message signal
    let error_signal = create_rw_signal(cx, None);

    // Create a readable state slice for the previous URL.
    let previous_url = create_read_slice(cx, state, |state| {
        state.runtime.as_ref().map(|r| r.previous_url().clone())
    });
    // Default to the home page if no previous URL is set.
    let redirect_url = previous_url().unwrap_or_default();
    let password_ref: NodeRef<Input> = create_node_ref(cx);

    let handle_submission = move |ev: SubmitEvent, user_defined: bool| {
        ev.prevent_default();
        let password = password_ref().expect("password to exist").value();
        // clone so original does not get moved into the closure
        let redirect_url = redirect_url.clone();

        spawn_local(async move {
            let vault_result = if user_defined {
                StringVault::new_and_validate(ROOT_USERNAME, &password).await
            } else {
                StringVault::new_and_create(ROOT_USERNAME, &password).await
            };

            match vault_result {
                Ok(string_vault) => {
                    set_vault(string_vault);
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

    let on_submit_user_defined = {
        let handle_submission = handle_submission.clone();
        move |ev: SubmitEvent| handle_submission(ev, true)
    };

    let on_submit_user_undefined = {
        let handle_submission = handle_submission.clone();
        move |ev: SubmitEvent| handle_submission(ev, false)
    };

    // check if user is defined
    create_effect(cx, move |_| {
        spawn_local({
            async move {
                let user_exists = StringVault::user_exists(ROOT_USERNAME).await;
                is_user_defined.set(user_exists);
                is_loading.set(false);
            }
        });
    });

    view! {
        cx,
        {move || if is_user_defined.get() {
            view! {
                cx,
                <div class="px-2 py-2">
                {form_view(cx, password_ref, on_submit_user_defined.clone(), SubmitButtonType::Login)}
                {move || if error_signal.get().is_some() {
                    view! { cx,
                       <div>
                           <div class="text-red-500">
                            { move || error_signal.get().unwrap_or("".to_string()) }
                            </div>
                            {reset_password_view(cx, is_user_defined.clone())}
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
                {form_view(cx, password_ref, on_submit_user_undefined.clone(), SubmitButtonType::Create("new password"))}
                </div>
            }
        }}
    }
}

fn form_view(
    cx: Scope,
    password_ref: NodeRef<Input>,
    on_submit: impl Fn(SubmitEvent) + 'static,
    button_type: SubmitButtonType,
) -> View {
    let placeholder = match button_type {
        SubmitButtonType::Create(_) => "Enter new password",
        _ => "Enter password",
    };

    let is_enabled = create_rw_signal(cx, true);

    view! {
        cx,
        <form class="flex flex-col w-96" on:submit=on_submit>
            <div class="flex flex-col mb-4">
                <input type="password"
                    placeholder={placeholder}
                    class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                    node_ref=password_ref
                />
            </div>

            <div class="flex flex-col items-start">
            <FormSubmitButton button_type=button_type button_enabled=is_enabled/>
            </div>
        </form>
    }.into_view(cx)
}


fn reset_password_view(
    cx: Scope,
    is_user_defined: RwSignal<bool>,
) -> View {
    let reset_vault = {
        let is_user_defined = is_user_defined.clone();
        move |ev: MouseEvent| {
            ev.prevent_default(); // needed to prevent form submission
            spawn_local({
                let is_user_defined = is_user_defined.clone();
                async move {
                    match StringVault::reset_vault(ROOT_USERNAME).await {
                        Ok(_) => {
                            log!("Vault reset successfully");
                            is_user_defined.set(false);
                        }
                        Err(err) => log!("Error resetting vault: {:?}", err),
                    }
                }
            });
        }
    };

    view! { cx,
        <button
            class=""
            on:click=reset_vault
        >
            "Reset Password"
        </button>
    }.into_view(cx)
}
