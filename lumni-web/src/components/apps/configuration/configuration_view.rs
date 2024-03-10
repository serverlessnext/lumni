use leptos::html::Input;
use leptos::logging::log;
use leptos::*;

use super::environment_configurations::EnvironmentConfigurations;
use crate::components::apps::configuration::AppConfig;
use crate::components::forms::FormStorage;
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
                    <ToggleButton show_delete_view=show_delete_view />
                    <hr/>
                    <ul>
                       <ItemList
                            show_delete_view=show_delete_view.get()
                            item_list=item_list
                            set_is_loading=set_is_loading
                            selected_item_id=selected_item_id
                        />
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
fn ToggleButton(show_delete_view: RwSignal<bool>) -> impl IntoView {
    view! {
        <button on:click=move |_| show_delete_view.set(!show_delete_view.get())>
            {if show_delete_view.get_untracked() {
                "Cancel"
            } else {
                "Delete"
            }}
        </button>
    }
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
        <li class="flex items-center space-x-4">
            <input type="radio" disabled={true} class="opacity-50" />
            <input class="px-4 py-1"
                placeholder="Bucket URI"
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
            <button class="px-4 py-1" on:click=move |_| {
                if let Some(name) = get_input_value(&input_ref) {
                    let app_config = AppConfig::new(&app_uri_clone_for_click, Some(&name), None).unwrap();
                    set_item_list.update(move |item_list| {
                        let profile_id = app_config.profile_id();
                        let _ = item_list.add(app_config, set_is_loading, set_submit_error);
                        selected_item_id.set(Some(profile_id));
                    });
                }
            }> "Add new" </button>
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
        <li class="flex items-center space-x-4">
            <input type="radio" name="selectedItem" value={profile_id.clone()}
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
    let set_item_list =
        use_context::<WriteSignal<EnvironmentConfigurations>>().unwrap();
    let profile_id = item.profile_id();
    let profile_name = item.profile_name();

    view! {
        <li class="flex items-center space-x-4 justify-between">
            <span class="text-blue-500 hover:text-blue-700">{profile_name}</span>
            <button class="ml-4 text-red-500 hover:text-red-700"
                on:click=move |_| {
                    let profile_id = profile_id.clone();
                    set_item_list.update(move |env_configs| {
                        env_configs.remove(profile_id.clone(), set_is_loading);
                    });
                }> "Delete" </button>
        </li>
    }
}
