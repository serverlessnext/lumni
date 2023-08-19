use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use leptos::log;

use crate::components::forms::{FormData, FormError};

pub trait FormStorage {
    fn list_items(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<ConfigurationFormMeta>, String>>
                + '_,
        >,
    >;
    fn load_content<'a>(
        &'a self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, String>> + '_>>;
    fn save_content<'a>(
        &'a self,
        form_meta: &ConfigurationFormMeta,
        content: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>>;
}

#[derive(Clone, Debug)]
pub struct ConfigurationFormMeta {
    id: String,
    name: Option<String>,
    template: Option<String>,
    tags: Option<HashMap<String, String>>, // original tags
}

#[allow(dead_code)]
impl ConfigurationFormMeta {
    pub fn new<S: Into<String>>(id: S, name: S, template: S) -> Self {
        Self {
            id: id.into(),
            name: Some(name.into()),
            template: Some(template.into()),
            tags: None,
        }
    }

    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            name: None,
            template: None,
            tags: None,
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn template(&self) -> Option<String> {
        self.template.clone()
    }

    pub fn tags(&self) -> Option<HashMap<String, String>> {
        self.tags.clone()
    }
}

#[derive(Clone, Debug)]
pub struct FormStorageHandler<S: FormStorage> {
    storage: S,
}

impl<S: FormStorage> FormStorageHandler<S> {
    pub fn new(storage: S) -> Self {
        FormStorageHandler { storage }
    }

    pub async fn get_form_info(
        &self,
        form_id: &str,
    ) -> Result<Option<HashMap<String, String>>, String> {
        let configurations = self.storage.list_items().await?;

        let form_data_option = configurations
            .iter()
            .find(|form_data| form_data.id() == form_id);

        match form_data_option {
            Some(form_data) => Ok(form_data.tags()),
            None => Err("Form data not found".to_string()),
        }
    }

    pub async fn get_configuration_meta(
        &self,
        form_id: &str,
    ) -> Result<ConfigurationFormMeta, String> {
        let tags_opt = self.get_form_info(form_id).await?;
        if let Some(tags) = tags_opt {
            let profile_name = tags.get("ConfigName").cloned();
            let template_name = tags.get("TemplateName").cloned();

            Ok(ConfigurationFormMeta {
                id: form_id.to_string(),
                name: profile_name,
                template: template_name,
                tags: Some(tags),
            })
        } else {
            Err("Form data not found".to_string())
        }
    }

    pub async fn load_config(
        &self,
        form_id: &str,
    ) -> Result<Option<HashMap<String, String>>, String> {
        let content_result = self.storage.load_content(form_id).await?;

        match content_result {
            Some(data) => {
                match serde_json::from_slice::<HashMap<String, String>>(&data) {
                    Ok(config) => Ok(Some(config)),
                    Err(e) => {
                        log::error!("error deserializing config: {:?}", e);
                        Err(e.to_string())
                    }
                }
            }
            None => Ok(None),
        }
    }

    pub async fn save_config(
        &self,
        form_data: &FormData,
    ) -> Result<(), FormError> {
        let form_config = form_data.export_config();

        // Serialize form data into JSON
        let document_content = match serde_json::to_vec(&form_config) {
            Ok(content) => content,
            Err(e) => {
                log::error!("error serializing config: {:?}", e);
                return Err(FormError::SubmitError(e.to_string()));
            }
        };

        let form_meta = form_data.meta_data();
        match self
            .storage
            .save_content(form_meta, &document_content)
            .await
        {
            Ok(_) => {
                log!("Successfully saved form data");
                Ok(())
            }
            Err(e) => {
                log!("Failed to save form data. Error: {:?}", e);
                Err(FormError::SubmitError(e.to_string()))
            }
        }
    }
}
