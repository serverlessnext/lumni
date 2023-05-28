
use leptos::ev::MouseEvent;
use leptos::*;

use crate::GlobalState;


#[component]
pub fn LogoutForm(cx: Scope) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    // Create writable state slices for the vault and initialization status.
    let set_vault =
        create_write_slice(cx, state, |state, vault| state.vault = vault);
    let set_vault_initialized =
        create_write_slice(cx, state, |state, initialized| {
            if let Some(runtime) = &mut state.runtime {
                runtime.set_vault_initialized(initialized);
            }
        });

    let on_click = move |ev: MouseEvent| {
        ev.prevent_default();
        log!("Logging out");
        set_vault(None);
        set_vault_initialized(false);
    };

    view! { cx,
        <button
            class="text-red-500 hover:text-red-700"
            on:click=on_click
        >
            "Log Out"
        </button>
    }
}
