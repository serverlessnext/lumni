use leptos::*;

use super::SubmitButtonType;

#[component]
pub fn FormSubmitButton(cx: Scope, button_type: SubmitButtonType, button_enabled: RwSignal<bool>) -> impl IntoView {
    let button_text = button_type.button_text();

    view! {
        cx,
        <button
            type="submit"
            class={move || button_type.button_class(!button_enabled.get())}
            disabled={move || !button_enabled.get()}
        >
            {button_text}
        </button>
    }
}



