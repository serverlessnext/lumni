use std::error::Error;

use url::Url;

#[derive(Clone)]
pub struct Endpoints {
    completion: Option<Url>,
    tokenizer: Option<Url>,
    settings: Option<Url>,
}

impl Endpoints {
    pub fn new() -> Self {
        Endpoints {
            completion: None,
            tokenizer: None,
            settings: None,
        }
    }

    pub fn get_completion_endpoint(&self) -> Result<String, Box<dyn Error>> {
        match self.completion.as_ref() {
            Some(url) => Ok(url.to_string()),
            None => Err("Completion endpoint not set".into()),
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

    pub fn set_settings(mut self, url: Url) -> Self {
        self.settings = Some(url);
        self
    }
}