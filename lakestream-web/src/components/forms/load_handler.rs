use leptos::*;

use super::form_data::FormData;

pub trait LoadHandler {
    fn is_loading(&self) -> RwSignal<bool>;
    fn load_error(&self) -> RwSignal<Option<String>>;
    fn form_data(&self) -> RwSignal<Option<FormData>>;
}
