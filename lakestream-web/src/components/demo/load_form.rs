use std::collections::HashMap;

use leptos::*;
use uuid::Uuid;

use super::dummy_data::make_form_data;
#[cfg(debug_assertions)]
#[cfg(feature = "debug-assertions")]
use super::helpers::debug_sleep;
use crate::components::forms::builders::{
    FormBuilder, FormType, LoadParameters,
};
use crate::components::forms::{ConfigurationFormMeta, FormData, FormError};

#[component]
pub fn LoadFormDemo(cx: Scope) -> impl IntoView {
    let is_loading = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    // define a function that fetches the data
    let handle_load = {
        let dummy_data = make_form_data(cx);
        move |form_data_rw: RwSignal<Option<FormData>>| {
            let dummy_data = dummy_data.clone();
            //is_loading.set(true);
            spawn_local(async move {
                // run data loading on the background
                let data = load_data().await;
                log!("Loaded data: {:?}", data);
                form_data_rw.set(Some(dummy_data));
                is_loading.set(false);
            });
        }
    };

    let load_parameters = LoadParameters::new(
        Some(Box::new(handle_load)),
        Some(is_loading),
        Some(validation_error),
    );

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let load_form = FormBuilder::new(
        "Load Form",
        form_meta,
        FormType::LoadData(load_parameters),
    )
    .build(cx, None);

    load_form.to_view()
}

async fn load_data() -> Result<HashMap<String, String>, FormError> {
    #[cfg(feature = "debug-assertions")]
    crate::debug_sleep!();

    Ok(HashMap::new())
}
