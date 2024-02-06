use leptos::*;

use super::submit_handler::SubmitHandler;
use super::FormData;

pub trait FormHandlerTrait {
    fn is_processing(&self) -> RwSignal<bool>;
    fn process_error(&self) -> RwSignal<Option<String>>;
    fn form_data(&self) -> RwSignal<Option<FormData>>;
    fn on_submit(&self) -> &dyn SubmitHandler;
}
