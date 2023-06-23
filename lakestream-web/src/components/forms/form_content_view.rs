use leptos::ev::SubmitEvent;
use leptos::*;

use crate::components::buttons::{ButtonType, FormSubmitButton};
use crate::components::form_input::{TextBoxView, FormState};

#[component]
pub fn FormContentView<'a>(
    cx: Scope,
    form_state: FormState,
    on_submit: Box<dyn Fn(SubmitEvent, Option<FormState>)>,
    is_submitting: RwSignal<bool>,
    button_type: &'a ButtonType,
) -> impl IntoView {
    let form_state_clone = form_state.clone();
    let form_changed = create_rw_signal(cx, false);
    let button_type = button_type.clone(); // temp clone -- should change FormSubmitButton
    view! {
        cx,
        <form class="flex flex-wrap w-full max-w-2xl text-white border p-4 font-mono"
            on:submit=move |ev| {
                is_submitting.set(true);
                on_submit(ev, Some(form_state.clone()))
            }
        >
            <For
                each= move || {form_state_clone.clone().into_iter().enumerate()}
                    key=|(index, _)| *index
                    view= move |cx, (_, (_, form_element_state))| {
                        view! {
                            cx,
                            <TextBoxView
                                form_element_state
                                input_changed={form_changed}
                            />
                        }
                    }
            />
            <FormSubmitButton button_type button_enabled=form_changed.into()/>
        </form>
    }.into_view(cx)
}
