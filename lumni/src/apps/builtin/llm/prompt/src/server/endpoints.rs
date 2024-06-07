
use std::error::Error;
use lumni::HttpClient;
use url::Url;

use crate::external as lumni;

// currently llama server based only
// TODO: support more endpoints, add ability to customize
pub const DEFAULT_TOKENIZER_ENDPOINT: &str = "http://localhost:8080/tokenize";
pub const DEFAULT_COMPLETION_ENDPOINT: &str =
    "http://localhost:8080/completion";
pub const DEFAULT_SETTINGS_ENDPOINT: &str = "http://localhost:8080/props";

#[derive(Clone)]
pub struct Endpoints {
    completion: Url,
    tokenizer: Url,
    settings: Url,
}

impl Endpoints {
    pub fn default() -> Result<Self, Box<dyn Error>> {
        let completion = Url::parse(DEFAULT_COMPLETION_ENDPOINT)?;
        let tokenizer = Url::parse(DEFAULT_TOKENIZER_ENDPOINT)?;
        let settings = Url::parse(DEFAULT_SETTINGS_ENDPOINT)?;

        Ok(Endpoints {
            completion,
            tokenizer,
            settings,
        })
    }

    pub fn get_completion(&self) -> &Url {
        &self.completion
    }

    pub fn get_tokenizer(&self) -> &Url {
        &self.tokenizer
    }

    pub fn get_settings(&self) -> &Url {
        &self.settings
    }
}
