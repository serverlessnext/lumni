use leptos::html::Input;
use leptos::*;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::utils::local_storage::{load_from_storage, save_to_storage};
use super::ObjectStore;

const LOCAL_STORAGE_KEY: &str = "OBJECT_STORES";

#[component]
pub fn ObjectStoreListView(cx: Scope) -> impl IntoView {
    let (item_list, set_item_list) = create_signal(cx, ObjectStoreList::new());
    provide_context(cx, set_item_list);

    let input_ref = create_node_ref::<Input>(cx);

    fn get_input_value(input_ref: NodeRef<Input>) -> Option<String> {
        let input = input_ref.get()?;
        let value = input.value().trim().to_string();
        if !value.is_empty() {
            input.set_value("");
            Some(value)
        } else {
            None
        }
    }

    fn create_object_store(uri: String) -> ObjectStore {
        ObjectStore::new(Uuid::new_v4(), uri)
    }

    create_effect(cx, move |_| {
        item_list.get().save_to_local_storage();
    });

    create_effect(cx, move |_| {
        if let Some(input) = input_ref.get() {
            request_animation_frame(move || {
                let _ = input.focus();
            });
        }
    });

    let input_ref_clone = input_ref.clone();
    view! { cx,
        <div>
            <input class="px-4 py-2"
                placeholder="Bucket URI"
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        if let Some(uri) = get_input_value(input_ref_clone.clone()) {
                            let new_item = create_object_store(uri);
                            set_item_list.update(|item_list| item_list.add(new_item));
                        }
                    }
                }
                node_ref=input_ref
            />
            <button class="px-4 py-2" on:click=move |_| {
                if let Some(uri) = get_input_value(input_ref_clone.clone()) {
                    let new_item = create_object_store(uri);
                    set_item_list.update(|item_list| item_list.add(new_item));
                }
            }> "Add Item" </button>
        </div>
        <div>
            <ul>
                <For
                    each={move || item_list.get().items.clone()}
                    key=|item| item.id
                    view=move |cx, item: ObjectStore| view! { cx, <ListItem item /> }
                />
            </ul>
        </div>
    }
}

#[component]
fn ListItem(cx: Scope, item: ObjectStore) -> impl IntoView {
    let set_item = use_context::<WriteSignal<ObjectStoreList>>(cx).unwrap();
    let item_id = item.id;
    let item_uri = item.uri;

    view! { cx,
        <li>
            <div class="px-4 py-2">
                <a href={format!("/object-stores/{}", item_id)}>
                    {item_uri.clone()}
                </a>
                " | "
                <button class="text-red-500 hover:text-red-700" on:click=move |_| set_item.update(|t| t.remove(item_id))> "delete" </button>
            </div>
        </li>
    }
}


#[derive(Debug, Clone)]
pub struct ObjectStoreList {
    pub items: Vec<ObjectStore>,
}

impl ObjectStoreList {
    pub fn new() -> Self {
        let initial_items = Self::load_from_local_storage();
        Self {
            items: initial_items,
        }
    }

    pub fn load_from_local_storage() -> Vec<ObjectStore> {
        load_from_storage::<Vec<ItemSerialized>>(LOCAL_STORAGE_KEY)
            .map(|values| {
                values
                    .into_iter()
                    .map(|stored| stored.into_item())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn save_to_local_storage(&self) {
        save_to_storage(
            LOCAL_STORAGE_KEY,
            &self
                .items
                .iter()
                .map(ItemSerialized::from)
                .collect::<Vec<_>>(),
        );
    }

    // Add and remove now operate on non-reactive types
    pub fn add(&mut self, item: ObjectStore) {
        self.items.push(item);
    }

    pub fn remove(&mut self, id: Uuid) {
        self.items.retain(|item| item.id != id);
    }
}

impl ItemSerialized {
    pub fn into_item(self) -> ObjectStore {
        ObjectStore {
            id: self.id,
            uri: self.uri,
        }
    }
}

impl From<&ObjectStore> for ItemSerialized {
    fn from(item: &ObjectStore) -> Self {
        Self {
            id: item.id,
            uri: item.uri.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ItemSerialized {
    pub id: Uuid,
    pub uri: String,
}
