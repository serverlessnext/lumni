use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use uuid::Uuid;

use crate::builders::{FieldBuilder, FormBuilder, FormParameters};
use crate::components::forms::{FormData, FormError};

#[cfg(debug_assertions)]
#[cfg(feature = "debug-assertions")]
async fn debug_sleep() {
    use std::time::Duration;

    #[cfg(feature = "debug-assertions")]
    use async_std::task;
    task::sleep(Duration::from_secs(1)).await;
}

#[cfg(feature = "debug-assertions")]
macro_rules! debug_sleep {
    () => {
        #[cfg(debug_assertions)]
        {
            debug_sleep().await;
        }
    };
}

#[component]
pub fn SearchForm(cx: Scope) -> impl IntoView {
    let is_submitting = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    let handle_search = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();

            log!("Button clicked");
            is_submitting.set(true);

            spawn_local(async move {
                // run search on background
                let data = extract_form_data(form_data)
                    .map_err(|e| {
                        log!("Error: {:?}", e);
                        validation_error
                            .set(Some("FORM_DATA_MISSING".to_string()));
                    })
                    .unwrap();
                #[cfg(feature = "debug-assertions")]
                debug_sleep!();

                log!("Form data: {:?}", data);
                is_submitting.set(false);
            });
        }
    };

    let form_parameters = FormParameters::new(
        Some(Box::new(handle_search)),
        Some(is_submitting),
        Some(validation_error),
    );

    let query_form = FormBuilder::new("Query", &Uuid::new_v4().to_string())
        .add_element(Box::new(
            FieldBuilder::new("Select")
                .with_label("Select")
                .as_input_field()
                .with_initial_value("*"),
        ))
        .add_element(Box::new(
            FieldBuilder::new("From")
                .with_label("From")
                .as_input_field()
                .with_initial_value("table"),
        ))
        .with_form_parameters(form_parameters)
        .build(cx);

    let results_form =
        FormBuilder::new("Search Form", &Uuid::new_v4().to_string())
            .add_element(Box::new(FieldBuilder::new("Query").as_input_field()))
            .build(cx);

    view! { cx,
        { query_form.to_view() }
        { move ||
            if is_submitting.get() {
                view! { cx, "" }.into_view(cx)
            } else if let Some(error) = validation_error.get() {
                view! { cx, <p>{ error }</p> }.into_view(cx)
            } else {
                view ! {
                    cx,
                    <div>
                        <p>"Results"</p>
                    </div>
                    { results_form.to_view() }
                }.into_view(cx)
            }
        }
    }
    .into_view(cx)
}

fn extract_form_data(
    form_data: Option<FormData>,
) -> Result<HashMap<String, String>, FormError> {
    let data = form_data
        .ok_or_else(|| FormError::SubmitError("FORM_DATA_MISSING".to_string()))
        .map(|data| data.to_hash_map())?;
    Ok(data)
}
