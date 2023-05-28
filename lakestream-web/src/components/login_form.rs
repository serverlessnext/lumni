use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::html::Input;
use leptos::*;
use leptos_router::use_navigate;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::stringvault::StringVault;
use crate::GlobalState;

const ROOT_USERNAME: &str = "admin";

#[component]
pub fn LoginForm(cx: Scope) -> impl IntoView {
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

    // Create an error message signal
    let error_signal = create_rw_signal(cx, None);

    // Create a readable state slice for the previous URL.
    let previous_url = create_read_slice(cx, state, |state| {
        state.runtime.as_ref().map(|r| r.previous_url().clone())
    });
    // Default to the home page if no previous URL is set.
    let redirect_url = previous_url().unwrap_or_default();
    let password_ref: NodeRef<Input> = create_node_ref(cx);

    // Define the form submission behavior.
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let password = password_ref().expect("password to exist").value();
        // clone so original does not get moved into the closure
        let redirect_url = redirect_url.clone();

        spawn_local(async move {
            match StringVault::new_and_validate(ROOT_USERNAME, &password).await
            {
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

    view! {
        cx,
         <form class="flex flex-col w-96" on:submit=on_submit.clone()>
             <div class="flex flex-col mb-4">
                 <label class="mb-2">"Password"</label>
                 <input type="password"
                     class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                     node_ref=password_ref
                 />
             </div>

             <button
                 type="submit"
                 class="bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
             >
                 "Log In"
             </button>
         </form>

         // show if there an error on password input
         {move || if error_signal.get().is_some() {
             view! { cx,
                <div>
                    <div class="text-red-500">
                     { move || error_signal.get().unwrap_or("".to_string()) }
                     </div>
                     <ResetPasswordButton />
                </div>
             }
         } else {
             view! { cx, <div></div> }
         }}
    }
}

#[component]
pub fn ResetPasswordButton(cx: Scope) -> impl IntoView {
    let reset_vault = move |ev: MouseEvent| {
        ev.prevent_default(); // needed to prevent form submission
        spawn_local(async move {
            match StringVault::reset_vault(ROOT_USERNAME).await {
                Ok(_) => log!("Vault reset successfully"),
                Err(err) => log!("Error resetting vault: {:?}", err),
            }
        });
    };

    view! { cx,
        <button
            class=""
            on:click=reset_vault
        >
            "Reset Password"
        </button>
    }
}
