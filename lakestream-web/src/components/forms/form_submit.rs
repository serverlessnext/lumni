use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};

use super::FormContentView;
use crate::components::form_helpers::FormSubmissionStatusView;
use crate::components::form_input::InputElements;

#[derive(Clone)]
pub struct FormSubmitData {
    input_elements: InputElements,
    meta_data: ItemMetaData,
}

impl FormSubmitData {
    pub fn new(input_elements: InputElements, meta_data: ItemMetaData) -> Self {
        Self {
            input_elements,
            meta_data,
        }
    }

    pub fn input_elements(&self) -> InputElements {
        self.input_elements.clone()
    }
}

pub trait FormSubmitHandler {
    fn on_submit(&self) -> Box<dyn Fn(SubmitEvent, InputElements) + 'static>;
    fn is_submitting(&self) -> RwSignal<bool>;
    fn submit_error(&self) -> RwSignal<Option<String>>;
    fn data(&self) -> &FormSubmitData;
}

pub struct FormSaveHandler {
    data: FormSubmitData,
    is_submitting: RwSignal<bool>,
    submit_error: RwSignal<Option<String>>,
    on_submit_fn: Rc<dyn Fn(SubmitEvent, InputElements) + 'static>,
}

impl FormSaveHandler {
    pub fn new(
        cx: Scope,
        vault: LocalEncrypt,
        data: FormSubmitData,
    ) -> Box<Self> {
        Box::new(Self::_new(cx, vault, data))
    }

    fn _new(cx: Scope, vault: LocalEncrypt, data: FormSubmitData) -> Self {
        let is_submitting = create_rw_signal(cx, false);
        let submit_error = create_rw_signal(cx, None::<String>);
        let meta_data = data.meta_data.clone();
        let vault = Rc::new(vault);

        let on_submit_fn: Rc<dyn Fn(SubmitEvent, InputElements)> =
            Rc::new(move |ev: SubmitEvent, input_elements: InputElements| {
                let vault = Rc::clone(&vault);
                ev.prevent_default();

                let mut validation_errors = HashMap::new();

                for (key, (_, _, value_signal, _)) in &input_elements {
                    let value = value_signal.get();
                    let validator = input_elements
                        .get(key)
                        .expect("Validator to exist")
                        .3
                        .validator
                        .clone();

                    if let Some(validator) = &validator {
                        if let Err(e) = validator(&value) {
                            log::error!("Validation failed: {}", e);
                            validation_errors
                                .insert(key.clone(), e.to_string());
                        }
                    }
                }

                // Write validation errors to corresponding WriteSignals
                for (key, (_, error_signal, _, _)) in &input_elements {
                    if let Some(error) = validation_errors.get(key) {
                        error_signal.set(Some(error.clone()));
                    } else {
                        error_signal.set(None);
                    }
                }

                if validation_errors.is_empty() {
                    let form_config: HashMap<String, String> = input_elements
                        .iter()
                        .map(|(key, (_, _, value_signal, _))| {
                            (key.clone(), value_signal.get())
                        })
                        .collect();

                    let document_content =
                        serde_json::to_vec(&form_config).unwrap();

                    // Use renamed function `submit_save_form_content`
                    submit_save_form_content(
                        vault,
                        meta_data.clone(),
                        document_content,
                        is_submitting.clone(),
                        submit_error.clone(),
                    );
                }
            });

        Self {
            data,
            is_submitting,
            submit_error,
            on_submit_fn,
        }
    }
}

impl FormSubmitHandler for FormSaveHandler {
    fn on_submit(&self) -> Box<dyn Fn(SubmitEvent, InputElements) + 'static> {
        let on_submit_fn = self.on_submit_fn.clone();
        Box::new(move |ev: SubmitEvent, input_elements: InputElements| {
            on_submit_fn(ev, input_elements)
        })
    }

    fn is_submitting(&self) -> RwSignal<bool> {
        self.is_submitting.clone()
    }

    fn submit_error(&self) -> RwSignal<Option<String>> {
        self.submit_error.clone()
    }

    fn data(&self) -> &FormSubmitData {
        &self.data
    }
}

pub fn submit_save_form_content(
    vault: Rc<LocalEncrypt>,
    meta_data: ItemMetaData,
    document_content: Vec<u8>,
    is_submitting: RwSignal<bool>,
    submit_error: RwSignal<Option<String>>,
) {
    let mut local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("Invalid browser storage type"))
        }
        _ => panic!("Invalid storage backend"),
    };

    spawn_local(async move {
        match local_storage
            .save_content(meta_data, &document_content)
            .await
        {
            Ok(_) => {
                log!("Successfully saved secure configuration",);
                is_submitting.set(false);
            }
            Err(e) => {
                log!("Failed to save secure configuration. Error: {:?}", e);
                submit_error.set(Some(e.to_string()));
                is_submitting.set(false);
            }
        };
    });
    log!("Saved items");
}

#[component]
pub fn SubmitFormView(
    cx: Scope,
    handler: Box<dyn FormSubmitHandler>,
) -> impl IntoView {
    let input_elements = handler.data().input_elements();
    let is_submitting = handler.is_submitting();
    let submit_error = handler.submit_error();
    let on_submit = handler.on_submit();

    view! {
        cx,
        <div>
            <FormContentView
                input_elements={input_elements}
                on_submit
                is_submitting
            />
            <FormSubmissionStatusView is_submitting={is_submitting.into()} submit_error={submit_error.into()} />
        </div>
    }
}
