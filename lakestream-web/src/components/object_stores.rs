use leptos::html::Input;
use leptos::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::local_storage::{load_from_storage, save_to_storage};

const LOCAL_STORAGE_KEY: &str = "OBJECT_STORE_URI_LIST";

#[derive(Debug, Clone)]
pub struct ObjectStoreList(pub Vec<ObjectStore>);

impl ObjectStoreList {
    pub fn new(cx: Scope) -> Self {
        let initial_items =
            load_from_storage::<Vec<ItemSerialized>>(LOCAL_STORAGE_KEY)
                .map(|values| {
                    values
                        .into_iter()
                        .map(|stored| stored.into_item(cx))
                        .collect()
                })
                .unwrap_or_default();
        Self(initial_items)
    }

    pub fn add(&mut self, item: ObjectStore) {
        self.0.push(item);
    }

    pub fn remove(&mut self, id: Uuid) {
        self.0.retain(|item| item.id != id);
    }
}

#[derive(Debug, Clone)]
pub struct ObjectStore {
    pub id: Uuid,
    pub uri: RwSignal<String>,
}

impl ObjectStore {
    pub fn new(cx: Scope, id: Uuid, uri: String) -> Self {
        let uri = create_rw_signal(cx, uri);
        Self { id, uri }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ItemSerialized {
    pub id: Uuid,
    pub uri: String,
}

impl ItemSerialized {
    pub fn into_item(self, cx: Scope) -> ObjectStore {
        ObjectStore::new(cx, self.id, self.uri)
    }
}

impl From<&ObjectStore> for ItemSerialized {
    fn from(item: &ObjectStore) -> Self {
        Self {
            id: item.id,
            uri: item.uri.get(),
        }
    }
}

#[component]
pub fn ObjectStoreConfigurator(cx: Scope) -> impl IntoView {
    let (item_list, set_item_list) =
        create_signal(cx, ObjectStoreList::new(cx));

    provide_context(cx, set_item_list);

    let input_ref = create_node_ref::<Input>(cx);
    let item_list_clone = item_list.clone();

    fn parse_input_item(
        cx: Scope,
        input_ref: NodeRef<Input>,
    ) -> Option<ObjectStore> {
        let input = input_ref.get().unwrap();
        let uri = input.value();
        let uri = uri.trim();
        if !uri.is_empty() {
            let new = ObjectStore::new(cx, Uuid::new_v4(), uri.to_string());
            input.set_value("");
            Some(new)
        } else {
            None
        }
    }

    create_effect(cx, move |_| {
        save_to_storage(
            LOCAL_STORAGE_KEY,
            &item_list_clone
                .get()
                .0
                .iter()
                .map(ItemSerialized::from)
                .collect::<Vec<_>>(),
        );
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
            <input
                placeholder="Bucket URI"
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        if let Some(new_item) = parse_input_item(cx, input_ref_clone.clone()) {
                            set_item_list.update(|item_list| item_list.add(new_item));
                        }
                    }
                }
                node_ref=input_ref
            />
            <button on:click=move |_| {
                if let Some(new_item) = parse_input_item(cx, input_ref_clone.clone()) {
                    set_item_list.update(|item_list| item_list.add(new_item));
                }
            }> "Add Item" </button>
        </div>
        <ul>
            <For
                each={move || item_list.get().0.clone()}
                key=|item| item.id
                view=move |cx, item: ObjectStore| view! { cx, <ListView item /> }
            />
        </ul>
    }
}

#[component]
fn ListView(cx: Scope, item: ObjectStore) -> impl IntoView {
    let set_list = use_context::<WriteSignal<ObjectStoreList>>(cx).unwrap();

    view! { cx,
        <li>
            <div>
                {move || item.uri.get()}
                " | "
                <button class="delete" on:click=move |_| set_list.update(|t| t.remove(item.id))> "delete" </button>
            </div>
        </li>
    }
}
