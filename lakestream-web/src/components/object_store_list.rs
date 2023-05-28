use leptos::html::Input;
use leptos::*;
use serde::{Deserialize, Serialize};

use crate::GlobalState;
use crate::stringvault::{StringVault, FormOwner, handle_form_submission};
use super::forms::object_store::ObjectStore;
use crate::utils::local_storage::{load_from_storage, save_to_storage};


const LOCAL_STORAGE_KEY: &str = "OBJECT_STORES";

#[component]
pub fn ObjectStoreListView(cx: Scope) -> impl IntoView {
    let (item_list, set_item_list) = create_signal(cx, ObjectStoreList::new());
    provide_context(cx, set_item_list);

    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let input_ref = create_node_ref::<Input>(cx);

    let (is_submitting, set_is_submitting) = create_signal(cx, false);
    let (submit_error, set_submit_error) = create_signal(cx, None::<String>);

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

    fn create_object_store(
        vault: StringVault,
        name: String,
        set_is_submitting: WriteSignal<bool>,
        set_submit_error: WriteSignal<Option<String>>,
    ) -> ObjectStore {
        let object_store = ObjectStore::new(name);
        let form_owner = FormOwner {
            tag: object_store.tag().to_uppercase(),
            id: object_store.id(),
        };
        let mut default_config = object_store.default_config();
        default_config.insert("AWS_ACCESS_KEY_ID".to_string(), "MY_AWS_ACCESS_KEY_ID".to_string());
        default_config.insert("AWS_SECRET_ACCESS_KEY".to_string(), "MY_AWS_SECRET_ACCESS_KEY".to_string());
        handle_form_submission(vault, form_owner, default_config, set_is_submitting, set_submit_error);
        object_store
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

    let vault_clone = vault.clone();
    let input_ref_clone = input_ref.clone();
    view! { cx,
        <div>
            <input class="px-4 py-2"
                placeholder="Bucket URI"
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        if let Some(name) = get_input_value(input_ref_clone.clone()) {
                            let new_item = create_object_store(vault_clone.clone(), name, set_is_submitting.clone(), set_submit_error.clone());
                            set_item_list.update(|item_list| item_list.add(new_item));
                        }
                    }
                }
                node_ref=input_ref
            />
            <button class="px-4 py-2" on:click=move |_| {
                if let Some(name) = get_input_value(input_ref_clone.clone()) {
                    let new_item = create_object_store(vault.clone(), name, set_is_submitting.clone(), set_submit_error.clone());
                    set_item_list.update(|item_list| item_list.add(new_item));
                }
            }> "Add Item" </button>
        </div>
        <div>
            <ul>
                <For
                    each={move || item_list.get().items.clone()}
                    key=|item| item.name.clone()
                    view=move |cx, item: ObjectStore| view! { cx, <ListItem item /> }
                />
            </ul>
        </div>
    }
}

#[component]
fn ListItem(cx: Scope, item: ObjectStore) -> impl IntoView {
    let set_item = use_context::<WriteSignal<ObjectStoreList>>(cx).unwrap();
    let item_id = item.id();
    let item_name = item.name;

    view! { cx,
        <li>
            <div class="px-4 py-2">
                <a href={format!("/object-stores/{}", item_id)}>
                    {item_name.clone()}
                </a>
                " | "
                <button class="text-red-500 hover:text-red-700" on:click=move |_| set_item.update(|t| t.remove(item_name.clone()))> "delete" </button>
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

    pub fn remove(&mut self, name: String) {
        self.items.retain(|item| item.name != name);
    }
}

impl ItemSerialized {
    pub fn into_item(self) -> ObjectStore {
        ObjectStore { name: self.name }
    }
}

impl From<&ObjectStore> for ItemSerialized {
    fn from(item: &ObjectStore) -> Self {
        Self {
            name: item.name.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ItemSerialized {
    pub name: String,
}
