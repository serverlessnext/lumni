use std::collections::HashMap;

use leptos::*;

use crate::components::apps::configuration::AppConfig;
use crate::components::forms::{ConfigurationFormMeta, FormError, FormStorage};

pub struct EnvironmentConfigurations {
    pub items: Vec<AppConfig>,
    pub storage: Box<dyn FormStorage>,
}

impl Clone for EnvironmentConfigurations {
    fn clone(&self) -> Self {
        EnvironmentConfigurations {
            items: self.items.iter().map(|item| item.clone()).collect(),
            storage: self.storage.clone(),
        }
    }
}

impl EnvironmentConfigurations {
    pub fn new(form_storage: Box<dyn FormStorage>) -> Self {
        Self {
            items: vec![],
            storage: form_storage,
        }
    }

    pub async fn load_from_vault(&self) -> Result<Vec<AppConfig>, FormError> {
        let configs =
            self.storage.list_items().await.map_err(FormError::from)?;

        let items: Vec<AppConfig> = configs
            .into_iter()
            .filter_map(|form_data| {
                let config_name = form_data.tags().and_then(|tags| {
                    tags.get("ProfileName")
                        .cloned()
                        .or_else(|| Some("Untitled".to_string()))
                });

                let app_uri = form_data
                    .tags()
                    .and_then(|tags| tags.get("AppName").cloned());

                app_uri
                    .and_then(|app_uri| {
                        config_name.map(|name| {
                            log!(
                                "Loaded name {} with template {}",
                                name,
                                app_uri
                            );
                            AppConfig::new(
                                app_uri,
                                Some(name),
                                Some(form_data.id()),
                            )
                        })
                    })
                    .ok_or_else(|| {
                        FormError::SubmitError(
                            "Form name not found".to_string(),
                        )
                    })
                    .ok()
                    .flatten()
            })
            .collect();
        Ok(items)
    }

    pub fn add(
        &mut self,
        item: AppConfig,
        set_is_submitting: WriteSignal<bool>,
        _set_submit_error: WriteSignal<Option<String>>,
    ) {
        set_is_submitting.set(true);

        let profile_name = item.profile_name();
        let profile_id = item.profile_id();
        let app_uri = item.app_uri();

        let mut tags = HashMap::new();
        tags.insert("ProfileName".to_string(), profile_name);
        tags.insert("AppName".to_string(), app_uri);

        let form_meta =
            ConfigurationFormMeta::with_id(&profile_id).with_tags(tags);

        spawn_local({
            let form_storage = self.storage.clone();
            async move {
                let _ = form_storage.add_item(&form_meta).await;
                set_is_submitting.set(false);
            }
        });
        self.items.push(item);
    }

    pub fn remove(
        &mut self,
        profile_id: String,
        set_is_loading: WriteSignal<bool>,
    ) {
        set_is_loading.set(true);
        spawn_local({
            let profile_id = profile_id.clone();
            let form_storage = self.storage.clone();
            async move {
                let _ = form_storage.delete_item(&profile_id).await;
                set_is_loading.set(false);
            }
        });

        self.items.retain(|item| item.profile_id() != profile_id);
    }
}
