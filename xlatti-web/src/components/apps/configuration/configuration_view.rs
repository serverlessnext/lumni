use leptos::html::Input;
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
            <div>"Error: Must be logged in to access Local Storage"</div>
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
    let selected_template = create_rw_signal("".to_string());

    let (is_loading, set_is_loading) = create_signal(true);
    let (item_list, set_item_list) = create_signal(
        EnvironmentConfigurations::new(storage_handler.clone()),
    );
    provide_context(set_item_list);

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
            let app_uri_clone_for_keydown = app_uri.clone();
            let app_uri_clone_for_click = app_uri.clone();
            let selected = selected_template.get();
            view! {
                <div>
                <p>{format!("Selected template: {}", selected)}</p>
                   <select on:change=move |ev| selected_template.set(event_target_value(&ev).into()) >
                       <option value="abc".to_string()>{"abc".to_string()}</option>
                       <option value="def".to_string()>{"def".to_string()}</option>
                       <option value="ghi".to_string()>{"ghi".to_string()}</option>
                   </select>
                <div>
                    <input class="px-4 py-2"
                        placeholder="Bucket URI"
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                if let Some(name) = get_input_value(input_ref) {
                                    let app_config = AppConfig::new(&app_uri_clone_for_keydown, Some(&name), None).unwrap();
                                    set_item_list.update(|item_list| item_list.add(app_config, set_is_loading, set_submit_error));
                                }
                            }
                        }
                        node_ref=input_ref
                    />
                    <button class="px-4 py-2" on:click=move |_| {
                        if let Some(name) = get_input_value(input_ref) {
                            let app_config =  AppConfig::new(&app_uri_clone_for_click, Some(&name), None).unwrap();
                            set_item_list.update(|item_list| item_list.add(app_config, set_is_loading, set_submit_error));
                        }
                    }> "Add Item" </button>
                </div>
                <div>
                    <ul>
                        <For
                            each={move || item_list.get().items}
                            key=|item| item.profile_id()
                            children=move |item| view! { <ListItem item set_is_loading/> }
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
    item: AppConfig,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView {
    let set_item =
        use_context::<WriteSignal<EnvironmentConfigurations>>().unwrap();
    let profile_id = item.profile_id();
    let profile_name = item.profile_name();

    view! {
        <li>
            <div class="px-4 py-2">
                {profile_name.clone()} " | "
                <a href={format!("/apps/{}/{}", item.app_uri(), profile_id)}>
                    "Form"
                </a>
                " | "
                <a href={format!("/apps/{}/{}?view=TextArea", item.app_uri(), profile_id)}>
                    "TextArea"
                </a>
                " | "
                <button class="text-red-500 hover:text-red-700" on:click=move |_| set_item.update(|t| t.remove(profile_id.clone(), set_is_loading))> "delete" </button>
            </div>
        </li>
    }
}
