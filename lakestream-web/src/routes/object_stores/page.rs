use leptos::html::Input;
use leptos::*;

use super::object_store::ObjectStore;
use crate::components::forms::form_handler::FormOwner;
use crate::stringvault::{SecureStringResult, StringVault};
use crate::GlobalState;

#[component]
pub fn ObjectStores(cx: Scope) -> impl IntoView {
    view! { cx,
        <ObjectStoreListView />
    }
}

#[component]
pub fn ObjectStoreListView(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let (is_loading, set_is_loading) = create_signal(cx, true);
    let (item_list, set_item_list) =
        create_signal(cx, ObjectStoreList::new(vault.clone()));
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

    let vault_clone = vault.clone();
    create_effect(cx, move |_| {
        spawn_local({
            let set_item_list = set_item_list.clone();
            let set_is_loading = set_is_loading.clone();
            let vault = vault_clone.clone();

            async move {
                let object_store_list = item_list.get_untracked();
                let initial_items = object_store_list
                    .load_from_vault()
                    .await
                    .unwrap_or_default();
                set_item_list.set(ObjectStoreList {
                    items: initial_items,
                    vault,
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
    pub vault: StringVault,
}

impl ObjectStoreList {
    pub fn new(vault: StringVault) -> Self {
        Self {
            items: vec![],
            vault,
        }
    }

    pub async fn load_from_vault(
        &self,
    ) -> SecureStringResult<Vec<ObjectStore>> {
        let configs = self.vault.list_configurations().await?;
        let items = configs
            .into_iter()
            .map(|(id, name)| ObjectStore::new_with_id(name, id))
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
        let form_owner = FormOwner {
            tag: "FORM".to_string(),
            id: object_store.id(),
        };

        spawn_local({
            let mut vault = self.vault.clone();
            async move {
                let _ = vault
                    .add_configuration(form_owner.to_object_key(), name)
                    .await;
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
        let form_owner = FormOwner::new_with_form_tag(item_id.clone());

        spawn_local({
            let mut vault = self.vault.clone();
            async move {
                let _ = vault
                    .delete_configuration(form_owner.to_object_key())
                    .await;
                set_is_loading.set(false);
            }
        });

        self.items.retain(|item| item.id() != item_id);
    }
}
