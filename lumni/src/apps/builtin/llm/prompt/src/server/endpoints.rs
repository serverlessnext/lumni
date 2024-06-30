use url::Url;

use crate::api::error::ApplicationError;

#[derive(Clone)]
pub struct Endpoints {
    completion: Option<Url>,
    tokenizer: Option<Url>,
    settings: Option<Url>,
    list_models: Option<Url>,
}

impl Endpoints {
    pub fn new() -> Self {
        Endpoints {
            completion: None,
            tokenizer: None,
            settings: None,
            list_models: None,
        }
    }

    pub fn get_completion_endpoint(&self) -> Result<String, ApplicationError> {
        match self.completion.as_ref() {
            Some(url) => Ok(url.to_string()),
            None => Err(ApplicationError::NotImplemented(
                "Completion endpoint not defined".to_string(),
            )),
        }
    }

    pub fn get_list_models_endpoint(&self) -> Result<String, ApplicationError> {
        match self.list_models.as_ref() {
            Some(url) => Ok(url.to_string()),
            None => Err(ApplicationError::NotImplemented(
                "List models endpoint not defined".to_string(),
            )),
        }
    }

    pub fn get_tokenizer(&self) -> Option<&Url> {
        self.tokenizer.as_ref()
    }

    pub fn get_settings(&self) -> Option<&Url> {
        self.settings.as_ref()
    }

    pub fn set_completion(mut self, url: Url) -> Self {
        self.completion = Some(url);
        self
    }

    pub fn set_tokenizer(mut self, url: Url) -> Self {
        self.tokenizer = Some(url);
        self
    }

    pub fn set_list_models(mut self, url: Url) -> Self {
        self.list_models = Some(url);
        self
    }

    pub fn set_settings(mut self, url: Url) -> Self {
        self.settings = Some(url);
        self
    }
}
