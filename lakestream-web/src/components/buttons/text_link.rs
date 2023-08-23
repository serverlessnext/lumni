use leptos::ev::MouseEvent;
use leptos::*;

use super::FormButton;

#[component]
pub fn TextLink<F>(
    cx: Scope,
    form_button: FormButton,
    enabled: Signal<bool>,
    on_click: F,
) -> impl IntoView
where
    F: Fn(MouseEvent) + 'static,
{
    let button_text = form_button.text();

    view! {
        cx,
        <button
            class={move || {
                form_button.clone().set_enabled(enabled.get()).button_class()
            }}
            on:click=on_click
            disabled=false
        >
            {button_text}
        </button>
    }
}
