use super::ModelFormatterTrait;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Generic {
    name: String,
    stop_tokens: Vec<String>,
}

impl Generic {
    pub fn new(name: &str) -> Self {
        Generic {
            name: name.to_string(),
            stop_tokens: vec![
                "### User: ".to_string(),
                "### Human: ".to_string(),
                "User: ".to_string(),
                "Human: ".to_string(),
            ],
        }
    }
}

impl ModelFormatterTrait for Generic {
    fn get_stop_tokens(&self) -> &Vec<String> {
        &self.stop_tokens
    }
}
