use leptos::ev::MouseEvent;
use leptos::*;

use super::FormButton;

#[component]
pub fn FormSubmitButton(
    cx: Scope,
    form_button: FormButton,
    button_enabled: Signal<bool>,
) -> impl IntoView {
    let button_text = form_button.text();

    view! {
        cx,
        <button
            type="submit"
            class={move || {
                let mut form_button = form_button.clone();
                form_button.set_enabled(!button_enabled.get());
                form_button.button_class()
            }}
            disabled={move || !button_enabled.get()}
        >
            {button_text}
        </button>
    }
}

#[component]
pub fn ClickButton<F>(
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
                let mut button_type = form_button.clone();
                button_type.set_enabled(!enabled.get());
                button_type.button_class()
            }}
            on:click=on_click
            disabled={!enabled.get()}
        >
            {button_text}
        </button>
    }
}
