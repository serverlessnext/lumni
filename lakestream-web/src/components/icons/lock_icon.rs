
use leptos::*;

#[component]
pub fn LockIconView(
    cx: Scope,
    is_locked: RwSignal<bool>,
    click_handler: Box<dyn Fn()>,
) -> impl IntoView {
    view! { cx,
        <div
            on:click=move |_| click_handler()
        >
            {move || if is_locked.get() {
                log!("is_locked");
                view! {cx, <LockIcon is_locked=true /> }
            } else {
                view! {cx, <LockIcon is_locked=false /> }
            }}
        </div>
    }
}

#[component]
fn LockIcon(cx: Scope, is_locked: bool) -> impl IntoView {
    if is_locked {
        view! {
            cx,
            <svg xmlns="http://www.w3.org/2000/svg" fill="white" viewBox="0 0 32 32" stroke-width="1" stroke="orange" class="w-10 h-10 py-1">
                <path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
            </svg>
        }
    } else {
        view! {
            cx,
            <svg xmlns="http://www.w3.org/2000/svg" fill="white" viewBox="0 0 32 32" stroke-width="1" stroke="green" class="w-10 h-10 py-1">
                <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 10.5V6.75a4.5 4.5 0 119 0v3.75M3.75 21.75h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H3.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
            </svg>
        }
    }
}
