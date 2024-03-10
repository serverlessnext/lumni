use leptos::html::Input;
use leptos::logging::log;
use leptos::*;

use super::environment_configurations::EnvironmentConfigurations;
use crate::components::apps::configuration::AppConfig;
use crate::components::forms::FormStorage;
use crate::components::icons::MenuToggleIcon;
use crate::helpers::local_storage::local_storage_handler;
use crate::routes::api::Login;

#[component]
pub fn AppConfiguration(app_uri: String) -> impl IntoView {
    let storage_handler = local_storage_handler();

    if let Some(storage_handler) = storage_handler {
        view! {
            <AppConfigurationView storage_handler app_uri/>
        }
    } else {
        view! {
            <div>"Login to access Local Storage"</div>
            <Login/>
        }
        .into_view()
    }
}

#[component]
pub fn AppConfigurationView(
    storage_handler: Box<dyn FormStorage>,
    app_uri: String,
) -> impl IntoView {
    let (is_loading, set_is_loading) = create_signal(true);
    let (item_list, set_item_list) =
        create_signal(EnvironmentConfigurations::new(storage_handler.clone()));
    provide_context(set_item_list);

    let selected_item_id = create_rw_signal(None::<String>);
    let show_delete_view = create_rw_signal(false);
    let input_ref = create_node_ref::<Input>();

    // TODO: implement error handling
    let (_, set_submit_error) = create_signal(None::<String>);

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

    create_effect(move |_| {
        spawn_local({
            let storage_handler = storage_handler.clone();
            async move {
                let object_store_list = item_list.get_untracked();
                let initial_items = object_store_list
                    .load_from_vault()
                    .await
                    .unwrap_or_default();
                set_item_list.set(EnvironmentConfigurations {
                    items: initial_items,
                    storage: storage_handler,
                });
                set_is_loading.set(false);
            }
        });
    });

    create_effect(move |_| {
        if let Some(input) = input_ref.get() {
            request_animation_frame(move || {
                let _ = input.focus();
            });
        }
    });

    view! {
        {move || if is_loading.get() {
            view! { <div>"Loading..."</div> }
        } else {
            view! {
                <div>
                    {log!("Selected: {:?}", selected_item_id.get_untracked())}
                    <div class="flex justify-end">
                        <MenuToggleIcon toggle_on=show_delete_view />
                    </div>
                    <ul>
                       <ItemList
                            show_delete_view=show_delete_view.get()
                            item_list=item_list
                            set_is_loading=set_is_loading
                            selected_item_id=selected_item_id
                        />
                        <hr class="my-4" />
                        {if !show_delete_view.get() {
                            view! {
                                <NewItemInput
                                    app_uri=app_uri.clone()
                                    set_item_list=set_item_list
                                    set_is_loading=set_is_loading
                                    set_submit_error=set_submit_error
                                    selected_item_id=selected_item_id
                                    input_ref=input_ref
                                />
                            }
                            .into_view()
                        } else {
                            view! { <li></li> }.into_view()
                        }}
                    </ul>
                </div>
            }
        }}
    }
}

#[component]
fn LoadingIndicator() -> impl IntoView {
    view! { <div>"Loading..."</div> }
}

#[component]
fn ItemList(
    show_delete_view: bool,
    item_list: ReadSignal<EnvironmentConfigurations>,
    set_is_loading: WriteSignal<bool>,
    selected_item_id: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <For
            each={move || item_list.get().items}
            key=|item| item.profile_id()
            children=move |item| {
                if show_delete_view {
                    view! { <ListItemDelete
                        item=item
                        set_is_loading=set_is_loading
                    /> }
                } else {
                    view! { <ListItem
                        item=item
                        selected_item_id=selected_item_id
                    /> }
                }
            }
        />
    }
}

#[component]
fn NewItemInput(
    app_uri: String,
    set_item_list: WriteSignal<EnvironmentConfigurations>,
    set_is_loading: WriteSignal<bool>,
    set_submit_error: WriteSignal<Option<String>>,
    selected_item_id: RwSignal<Option<String>>,
    input_ref: NodeRef<Input>,
) -> impl IntoView {
    let app_uri_clone_for_keydown = app_uri.clone();
    let app_uri_clone_for_click = app_uri.clone();

    view! {
        <li class="flex items-center space-x-4 mt-2">
            <input class="px-3 py-2 text-sm border rounded-md"
                placeholder="Profile Name"
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        if let Some(name) = get_input_value(&input_ref) {
                            let app_config = AppConfig::new(&app_uri_clone_for_keydown, Some(&name), None).unwrap();
                            set_item_list.update(move |item_list| {
                                let profile_id = app_config.profile_id();
                                let _ = item_list.add(app_config, set_is_loading, set_submit_error);
                                selected_item_id.set(Some(profile_id));
                            });
                        }
                    }
                }
                node_ref=input_ref
            />
            <button class="flex items-center justify-center p-1 rounded-full text-white bg-blue-500 hover:bg-blue-600" on:click=move |_| {
                if let Some(name) = get_input_value(&input_ref) {
                    let app_config = AppConfig::new(&app_uri_clone_for_click, Some(&name), None).unwrap();
                    set_item_list.update(move |item_list| {
                        let profile_id = app_config.profile_id();
                        let _ = item_list.add(app_config, set_is_loading, set_submit_error);
                        selected_item_id.set(Some(profile_id));
                    });
                }
            }>
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                </svg>
            </button>

        </li>
    }
}


fn get_input_value(input_ref: &NodeRef<Input>) -> Option<String> {
    input_ref
        .get()
        .map(|input| input.value().trim().to_string())
        .filter(|value| !value.is_empty())
}

#[component]
fn ListItem(
    item: AppConfig,
    selected_item_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let profile_id = item.profile_id();
    let profile_name = item.profile_name();

    let profile_id_clone = profile_id.clone(); // for first closure
    view! {
        <li class="flex items-center space-x-4 mb-1">
            <input type="radio" name="selectedItem" class="custom-radio" value={profile_id.clone()}
                checked={selected_item_id.get_untracked() == Some(item.profile_id())}
                on:change=move |_| {
                    log!("selected_item_id: {:?}", profile_id_clone);
                    selected_item_id.set(Some(profile_id_clone.clone()))
                }
            />
            <div>
                <a href={format!("/apps/{}/{}?view=TextArea", item.app_uri(), &profile_id)}
                    class="text-blue-500 hover:text-blue-700">
                    {profile_name}
                </a>
           </div>
        </li>
    }
}

#[component]
fn ListItemDelete(
    item: AppConfig,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView {
    let set_item_list = use_context::<WriteSignal<EnvironmentConfigurations>>().unwrap();
    let profile_id = item.profile_id();
    let profile_name = item.profile_name();

    view! {
        <li class="flex items-center space-x-4 mb-1">
            <div class="cursor-pointer" on:click=move |_| {
                let profile_id_clone = profile_id.clone();
                set_item_list.update(move |env_configs| {
                    env_configs.remove(profile_id_clone, set_is_loading);
                });
            }>
                // Trash icon SVG
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" class="w-6 h-6 text-red-500 hover:text-red-700">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                </svg>
            </div>
            <span class="text-slate-600">{profile_name}</span>
        </li>
    }
}



