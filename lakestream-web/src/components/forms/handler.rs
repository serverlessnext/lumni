use leptos::*;
use localencrypt::LocalEncrypt;

use super::load_handler::{LoadHandler, LoadVaultHandler};
use super::submit_handler::SubmitHandler;
use super::form_data::FormData;
use super::HtmlForm;


pub struct FormHandler {
    on_load: Option<Box<dyn LoadHandler>>,
    on_submit: Box<dyn SubmitHandler>,
}

impl FormHandler {
    pub fn new(
        on_load: Option<Box<dyn LoadHandler>>,
        on_submit: Box<dyn SubmitHandler>,
    ) -> Self {
        Self { on_load, on_submit }
    }

    pub fn new_with_vault(
        cx: Scope,
        form: HtmlForm,
        vault: &LocalEncrypt,
        submit_handler_factory: Box<
            dyn Fn(
                Scope,
                Option<&LocalEncrypt>,
                RwSignal<Option<FormData>>,
            ) -> Box<dyn SubmitHandler>,
        >,
    ) -> Self {
        let vault_handler = LoadVaultHandler::new(cx, form, vault);
        let form_data = vault_handler.form_data();
        let on_load: Option<Box<dyn LoadHandler>> = Some(vault_handler);

        let on_submit = submit_handler_factory(cx, Some(vault), form_data);

        Self { on_load, on_submit }
    }

    pub fn on_submit(&self) -> &dyn SubmitHandler {
        &*self.on_submit
    }

    pub fn on_load(&self) -> Option<&dyn LoadHandler> {
        self.on_load.as_ref().map(|handler| &**handler)
    }

    pub fn is_submitting(&self) -> RwSignal<bool> {
        self.on_submit.is_submitting()
    }

    pub fn submit_error(&self) -> RwSignal<Option<String>> {
        self.on_submit.submit_error()
    }
}
