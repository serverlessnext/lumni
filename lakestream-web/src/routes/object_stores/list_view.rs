use std::collections::HashMap;

use leptos::html::Input;
use leptos::*;
use localencrypt::{ItemMetaData, LocalStorage, SecureStringResult};

use super::object_store::ObjectStore;
use crate::GlobalState;

#[component]
pub fn ObjectStoreListView(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("Invalid browser storage type"))
        }
        _ => panic!("Invalid storage backend"),
    };

    let (is_loading, set_is_loading) = create_signal(cx, true);
    let (item_list, set_item_list) =
        create_signal(cx, ObjectStoreList::new(local_storage));
    provide_context(cx, set_item_list);

    let input_ref = create_node_ref::<Input>(cx);

    // TODO: implement error handling
    let (_, set_submit_error) = create_signal(cx, None::<String>);

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

    create_effect(cx, move |_| {
        spawn_local({
            let set_item_list = set_item_list.clone();
            let set_is_loading = set_is_loading.clone();

            let local_storage = match vault.backend() {
                localencrypt::StorageBackend::Browser(browser_storage) => {
                    browser_storage.local_storage().unwrap_or_else(|| {
                        panic!("Invalid browser storage type")
                    })
                }
                _ => panic!("Invalid storage backend"),
            };

            async move {
                let object_store_list = item_list.get_untracked();
                let initial_items = object_store_list
                    .load_from_vault()
                    .await
                    .unwrap_or_default();
                set_item_list.set(ObjectStoreList {
                    items: initial_items,
                    local_storage,
                });
                set_is_loading.set(false);
            }
        });
    });

    create_effect(cx, move |_| {
        if let Some(input) = input_ref.get() {
            request_animation_frame(move || {
                let _ = input.focus();
            });
        }
    });

    let input_ref_clone = input_ref.clone();

    view! {
        cx,
        {move || if is_loading.get() {
            view! { cx, <div>"Loading..."</div> }
        } else {
            view! {
                cx,
                <div>
                <div>
                    <input class="px-4 py-2"
                        placeholder="Bucket URI"
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                if let Some(name) = get_input_value(input_ref_clone.clone()) {
                                    set_item_list.update(|item_list| item_list.add(name, set_is_loading, set_submit_error));
                                }
                            }
                        }
                        node_ref=input_ref
                    />
                    <button class="px-4 py-2" on:click=move |_| {
                        if let Some(name) = get_input_value(input_ref_clone.clone()) {
                            set_item_list.update(|item_list| item_list.add(name, set_is_loading, set_submit_error));
                        }
                    }> "Add Item" </button>
                </div>
                <div>
                    <ul>
                        <For
                            each={move || item_list.get().items.clone()}
                            key=|item| item.name()
                            view=move |cx, item: ObjectStore| view! { cx, <ListItem item set_is_loading/> }
                        />
                    </ul>
                </div>
                </div>
            }
        }}
    }
}

#[component]
fn ListItem(
    cx: Scope,
    item: ObjectStore,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView {
    let set_item = use_context::<WriteSignal<ObjectStoreList>>(cx).unwrap();
    let item_id = item.id();
    let item_name = item.name();

    view! { cx,
        <li>
            <div class="px-4 py-2">
                <a href={format!("/object-stores/{}", item_id)}>
                    {item_name.clone()}
                </a>
                " | "
                <button class="text-red-500 hover:text-red-700" on:click=move |_| set_item.update(|t| t.remove(item_id.clone(), set_is_loading.clone()))> "delete" </button>
            </div>
        </li>
    }
}

#[derive(Debug, Clone)]
pub struct ObjectStoreList {
    pub items: Vec<ObjectStore>,
    pub local_storage: LocalStorage,
}

impl ObjectStoreList {
    pub fn new(local_storage: LocalStorage) -> Self {
        Self {
            items: vec![],
            local_storage,
        }
    }

    pub async fn load_from_vault(
        &self,
    ) -> SecureStringResult<Vec<ObjectStore>> {
        let configs = self.local_storage.list_items().await?;
        let items = configs
            .into_iter()
            .map(|form_data| {
                ObjectStore::new_with_id(
                    form_data
                        .tags()
                        .unwrap()
                        .get("Name")
                        .unwrap_or(&"Untitled".to_string())
                        .clone(),
                    form_data.id(),
                )
            })
            .collect();
        Ok(items)
    }

    pub fn add(
        &mut self,
        name: String,
        set_is_submitting: WriteSignal<bool>,
        _set_submit_error: WriteSignal<Option<String>>,
    ) {
        set_is_submitting.set(true);

        let object_store = ObjectStore::new(name.clone());

        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), name.clone());
        let meta_data = ItemMetaData::new_with_tags(&object_store.id(), tags);

        spawn_local({
            let mut local_storage = self.local_storage.clone();
            async move {
                let _ = local_storage.add_item(meta_data).await;
                set_is_submitting.set(false);
            }
        });
        self.items.push(object_store);
    }

    pub fn remove(
        &mut self,
        item_id: String,
        set_is_loading: WriteSignal<bool>,
    ) {
        set_is_loading.set(true);
        spawn_local({
            let item_id = item_id.clone();
            let mut local_storage = self.local_storage.clone();
            async move {
                let _ = local_storage.delete_item(&item_id).await;
                set_is_loading.set(false);
            }
        });

        self.items.retain(|item| item.id() != item_id);
    }
}
