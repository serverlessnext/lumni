use leptos::*;

#[component]
pub fn MenuToggleIcon(toggle_on: RwSignal<bool>) -> impl IntoView {
    view! {
        <div
            class="cursor-pointer"
            on:click=move |_| toggle_on.set(!toggle_on.get())
        >
            {if toggle_on.get() {
                // X icon to close the "on" state
                view! {
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" class="w-6 h-6">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                }
            } else {
                // Hamburger icon for the "off" state
                view! {
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" class="w-6 h-6">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7"/>
                    </svg>
                }
            }}
        </div>
    }
}
