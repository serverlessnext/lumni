use std::collections::HashMap;
use std::rc::Rc;

use leptos::*;
use localencrypt::LocalEncrypt;

use super::form_handler::{FormHandler, SubmitFormView};
use super::form_submit_handler::{FormSaveHandler, FormSubmitHandler};
use crate::components::form_input::FormElement;

#[derive(Clone, Debug)]
pub struct HtmlForm {
    name: String,
    id: String,
    elements: Vec<FormElement>,
}

impl HtmlForm {
    pub fn new(name: &str, id: &str, elements: Vec<FormElement>) -> Self {
        Self {
            name: name.to_string(),
            id: id.to_string(),
            elements,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn elements(&self) -> Vec<FormElement> {
        self.elements.clone()
    }

    pub fn default_field_values(&self) -> HashMap<String, String> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                FormElement::InputField(field_data) => {
                    Some((field_data.name.clone(), field_data.value.clone()))
                }
            })
            .collect()
    }
}

struct HtmlFormHandler {
    handler: Rc<FormHandler>,
}

impl HtmlFormHandler {
    pub fn new(handler: FormHandler) -> Self {
        let handler = Rc::new(handler);
        Self { handler }
    }

    pub fn create_view(&self, cx: Scope) -> View {
        let handler = Rc::clone(&self.handler);
        let is_submitting_signal = handler.is_submitting();
        let submit_error_signal = handler.submit_error();
        let form_submit_data_signal = handler.on_submit().data();

        view! { cx,
            {move ||
                if let Some(form_submit_data) = form_submit_data_signal.get() {
                    let handler = handler.clone();
                    view! {
                        cx,
                        <div>
                            <SubmitFormView handler=&*handler form_submit_data/>
                        </div>
                    }.into_view(cx)
                }
                else if let Some(error) = submit_error_signal.get() {
                    view! {
                        cx,
                        <div>
                            {"Error loading configuration: "}
                            {error}
                        </div>
                    }.into_view(cx)
                }
                else if is_submitting_signal.get() {
                    view! {
                        cx,
                        <div>
                            "Submitting..."
                        </div>
                    }.into_view(cx)
                }
                else {
                    view! {
                        cx,
                        <div>
                            "Loading..."
                        </div>
                    }.into_view(cx)
                }
            }
        }
        .into_view(cx)
    }
}

pub struct SaveFormHandler {
    cx: Scope,
    html_form_handler: HtmlFormHandler,
}

impl SaveFormHandler {
    pub fn new(cx: Scope, form: HtmlForm, vault: &LocalEncrypt) -> Self {
        let submit_handler = Box::new(
            move |_cx: Scope,
                  vault: Option<&LocalEncrypt>,
                  form_data: RwSignal<Option<super::FormSubmitData>>|
                  -> Box<dyn FormSubmitHandler> {
                // Ensure vault is available
                if let Some(_vault) = vault {
                    FormSaveHandler::new(_cx, _vault, form_data)
                } else {
                    panic!("Vault is required for SaveFormHandler");
                }
            },
        );

        let form_handler = FormHandler::new_with_vault(
            cx.clone(),
            form,
            &*vault,
            submit_handler,
        );
        let html_form_handler = HtmlFormHandler::new(form_handler);

        Self {
            cx,
            html_form_handler,
        }
    }

    pub fn create_view(&self) -> View {
        self.html_form_handler.create_view(self.cx)
    }
}
