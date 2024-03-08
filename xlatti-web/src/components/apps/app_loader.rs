use leptos::*;
use leptos::ev::MouseEvent;

use super::configuration::AppConfiguration;
use super::form_submit::AppFormSubmit;

#[component]
pub fn AppRunTime(app_uri: String) -> impl IntoView {
    view! { <AppFormSubmit app_uri/> }
}

#[component]
pub fn AppLoader(app_uri: String) -> impl IntoView {
    // TODO:
    // add Logger that can be toggled open/close, this should show stdout/stderr
    
    let is_enabled = create_rw_signal(false);
    let toggle_enabled = move |event: MouseEvent| {
        event.prevent_default();
        is_enabled.set(!is_enabled.get());
    };

    let app_uri_clone = app_uri.clone();
    view! {
        <div class="flex flex-col items-start max-w-2xl">
            <div 
                class="w-full px-2 py-1 bg-gray-200 flex justify-between items-center"
            >
                <span class="flex-grow"></span> {/* pushes toggle to the right */}
                <div
                    class="cursor-pointer hover:bg-gray-300 rounded p-1"
                    on:click=toggle_enabled
                >
                    {move || 
                        if is_enabled.get() {
                            view! { <span>{"-"}</span> }
                        } else {
                            view! { <span>{"+"}</span> }
                        }
                    }
                </div>
            </div>

            {move || 
                if is_enabled.get() {
                    view! {
                        <div class="bg-yellow-100 p-4 rounded-lg shadow my-4 w-full">
                            <AppConfiguration app_uri=app_uri.clone()/>
                        </div>
                    }
                } else {
                    view! {
                        <div class="max-h-0 w-full">
                            // This div remains collapsed
                        </div>
                    }
                }
            }
            <div class="w-full">
                <AppRunTime app_uri=app_uri_clone/>
            </div>
        </div>
    }

}
