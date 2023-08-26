use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use super::form_storage::{ConfigurationFormMeta, FormStorage};

#[derive(Clone, Debug)]
pub struct MemoryStorage {
    items: RefCell<HashMap<String, ConfigurationFormMeta>>,
    content: RefCell<HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            items: RefCell::new(HashMap::new()),
            content: RefCell::new(HashMap::new()),
        }
    }
}

impl FormStorage for MemoryStorage {
    fn list_items(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ConfigurationFormMeta>, String>>>>
    {
        let items = self.items.borrow().values().cloned().collect();
        Box::pin(async { Ok(items) })
    }

    fn load_content(
        &self,
        id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, String>>>> {
        let content = self.content.borrow().get(id).cloned();
        Box::pin(async { Ok(content) })
    }

    fn save_content(
        &self,
        form_meta: &ConfigurationFormMeta,
        content: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        self.items
            .borrow_mut()
            .insert(form_meta.id(), form_meta.clone());
        self.content
            .borrow_mut()
            .insert(form_meta.id(), content.to_vec());
        Box::pin(async { Ok(()) })
    }
}
