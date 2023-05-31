pub enum SubmitButtonType {
    Create(&'static str),
    Login,
    Save(&'static str),
}

impl SubmitButtonType {
    pub fn button_text(&self) -> String {
        match self {
            SubmitButtonType::Create(text) => format!("Create {}", text),
            SubmitButtonType::Login => "Log In".to_string(),
            SubmitButtonType::Save(text) => format!("Save {}", text),
        }
    }

    pub fn button_class(&self) -> &'static str {
        match self {
            SubmitButtonType::Create(_) => {
                "inline-block px-3 bg-orange-600 hover:bg-orange-700 \
                 text-white font-bold py-2 rounded"
            }
            SubmitButtonType::Login => {
                "inline-block px-3 bg-blue-600 hover:bg-blue-700 text-white \
                 font-bold py-2 rounded"
            }
            SubmitButtonType::Save(_) => {
                "inline-block px-3 bg-yellow-600 hover:bg-yellow-700 \
                 text-white font-bold py-2 rounded"
            }
        }
    }
}
