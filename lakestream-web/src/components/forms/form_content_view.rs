use leptos::ev::SubmitEvent;
use leptos::*;

use crate::components::buttons::{ButtonType, FormSubmitButton};
use crate::components::form_input::{InputBoxView, InputElements};

#[component]
pub fn FormContentView(
    cx: Scope,
    input_elements: InputElements,
    on_submit: Box<dyn Fn(SubmitEvent, InputElements)>,
    is_submitting: RwSignal<bool>,
) -> impl IntoView {
    let input_elements_clone = input_elements.clone();
    let form_changed = create_rw_signal(cx, false);
    view! {
        cx,
        <form class="flex flex-wrap w-full max-w-2xl text-white border p-4 font-mono"
            on:submit=move |ev| {
                is_submitting.set(true);
                on_submit(ev, input_elements.clone())
            }
        >
        <For
            each= move || {input_elements_clone.clone().into_iter().enumerate()}
                key=|(index, _input)| *index
                view= move |cx, (_, (label_text, input_element))| {
                    view! {
                        cx,
                        <InputBoxView
                            label_text
                            input_element
                            input_changed={form_changed}
                        />
                    }
                }
        />
        <FormSubmitButton button_type=ButtonType::Save(Some("Save Changes".to_string())) button_enabled=form_changed.into()/>
        </form>
    }
}
