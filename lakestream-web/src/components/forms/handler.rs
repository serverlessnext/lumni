use leptos::*;

use super::form_data::FormData;
use super::submit_handler::SubmitHandler;

pub trait FormHandlerTrait {
    fn is_processing(&self) -> RwSignal<bool>;
    fn process_error(&self) -> RwSignal<Option<String>>;
    fn form_data(&self) -> RwSignal<Option<FormData>>;
    fn on_submit(&self) -> &dyn SubmitHandler;
}
