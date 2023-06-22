use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};

use super::form_data::{FormData, SubmitInput};
use super::handler::FormHandler;
use super::save_handler::SaveHandler;
use super::submit_form_view::SubmitFormView;
use super::submit_handler::{CustomSubmitHandler, SubmitHandler};
use crate::components::buttons::ButtonType;
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

    pub fn create_view(&self, cx: Scope, button_type: ButtonType) -> View {
        let handler = Rc::clone(&self.handler);
        let is_submitting_signal = handler.is_submitting();
        let submit_error_signal = handler.submit_error();
        let form_submit_data_signal = handler.on_submit().data();
        let button_type = Rc::new(button_type);

        view! { cx,
            {move ||
                if let Some(form_submit_data) = form_submit_data_signal.get() {
                    let handler = handler.clone();
                    view! {
                        cx,
                        <div>
                            <SubmitFormView handler=&*handler form_submit_data button_type=&*button_type/>
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
                  form_data: RwSignal<Option<FormData>>|
                  -> Box<dyn SubmitHandler> {
                // Ensure vault is available
                if let Some(_vault) = vault {
                    SaveHandler::new(_cx, _vault, form_data)
                } else {
                    panic!("Vault is required for SaveFormHandler");
                }
            },
        );

        let form_handler =
            FormHandler::new_with_vault(cx, form, &*vault, submit_handler);
        let html_form_handler = HtmlFormHandler::new(form_handler);

        Self {
            cx,
            html_form_handler,
        }
    }

    pub fn create_view(&self) -> View {
        self.html_form_handler.create_view(
            self.cx,
            ButtonType::Save(Some("Save Changes".to_string())),
        )
    }
}

pub struct CustomFormHandler {
    cx: Scope,
    html_form_handler: HtmlFormHandler,
    button_type: Option<ButtonType>,
}

impl CustomFormHandler {
    pub fn new(
        cx: Scope,
        form: HtmlForm,
        function: Box<dyn Fn(SubmitEvent, Option<FormData>) + 'static>,
        is_submitting: RwSignal<bool>,
        submit_error: RwSignal<Option<String>>,
        button_type: Option<ButtonType>,
    ) -> Self {
        let default_field_values = form.default_field_values();
        let form_elements = form.elements();

        // Create MetaData (you may need to adjust this for your needs)
        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), form.name());
        let meta_data = ItemMetaData::new_with_tags(&form.id(), tags);

        let form_data_default = FormData::create_from_elements(
            cx,
            meta_data,
            &default_field_values,
            &form_elements,
        );

        let form_data = create_rw_signal(cx, Some(form_data_default));

        let custom_submit_handler = CustomSubmitHandler::new(
            form_data.clone(),
            Rc::new(
                move |ev: SubmitEvent, _submit_input: Option<SubmitInput>| {
                    function(ev, form_data.get());
                },
            ),
            is_submitting,
            submit_error,
        );

        let form_handler =
            FormHandler::new(None, Box::new(custom_submit_handler));
        let html_form_handler = HtmlFormHandler::new(form_handler);

        Self {
            cx,
            html_form_handler,
            button_type,
        }
    }

    pub fn create_view(&self) -> View {
        let button_type =
            self.button_type.clone().unwrap_or(ButtonType::Submit(None));
        self.html_form_handler.create_view(self.cx, button_type)
    }
}
