
use leptos::*;
use leptos::ev::MouseEvent;
use super::ButtonType;

#[component]
pub fn FormSubmitButton(
    cx: Scope,
    button_type: ButtonType,
    button_enabled: Signal<bool>,
) -> impl IntoView {
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

#[component]
pub fn ClickButton<F>(
    cx: Scope,
    button_type: ButtonType,
    enabled: Signal<bool>,
    on_click: F,
) -> impl IntoView
where
    F: Fn(MouseEvent) + 'static,
{
    let button_text = button_type.button_text();

    view! {
        cx,
        <button
            class={move || button_type.button_class(!enabled.get())}
            on:click=on_click
            disabled={!enabled.get()}
        >
            {button_text}
        </button>
    }
}

