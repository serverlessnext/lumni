use std::collections::HashMap;
use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::*;
use regex::Regex;
use uuid::Uuid;

use crate::builders::{FieldBuilder, FormBuilder, FormType, SubmitParameters};
use crate::components::form_input::validate_with_pattern;
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

    // define results_form first as its the target for handle_search
    let results_form = FormBuilder::new(
        "Search Form",
        &Uuid::new_v4().to_string(),
        None,
        FormType::LoadElements,
    )
    //.add_element(Box::new(FieldBuilder::new("Query").as_input_field()))
    .build(cx);

    // allows to overwrite the form
    let results_rw = results_form.form_data_rw();

    let handle_search = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();
            results_rw.set(None);
            is_submitting.set(true);

            spawn_local(async move {
                // run search on background
                let data = extract_form_data(form_data.clone())
                    .map_err(|e| {
                        log!("Error: {:?}", e);
                        validation_error
                            .set(Some("FORM_DATA_MISSING".to_string()));
                    })
                    .unwrap();
                #[cfg(feature = "debug-assertions")]
                debug_sleep!();

                log!("Form data: {:?}", data);
                if form_data.is_some() {
                    results_rw.set(form_data);
                }
                is_submitting.set(false);
            });
        }
    };

    let query_pattern = Regex::new(r"^test$").unwrap();

    let submit_parameters = SubmitParameters::new(
        Box::new(handle_search),
        Some(is_submitting),
        Some(validation_error),
        None,
    );

    let query_form = FormBuilder::new(
        "Query",
        &Uuid::new_v4().to_string(),
        None,
        FormType::SubmitData(submit_parameters),
    )
    .add_element(
        FieldBuilder::new("Select")
            .with_label("Select")
            .as_input_field()
            .with_initial_value("*"),
    )
    .add_element(
        FieldBuilder::new("From")
            .with_label("From")
            .as_input_field()
            .with_initial_value("table")
            .validator(Some(Arc::new(validate_with_pattern(
                query_pattern,
                "Invalid key.".to_string(),
            )))),
    )
    .build(cx);

    view! { cx,
        { query_form.to_view() }
        { move ||
            if results_rw.get().is_none() {
                // submit not yet clicked
                view! { cx, ""}.into_view(cx)
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
