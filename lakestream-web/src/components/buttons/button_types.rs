
#[derive(Clone)]
pub enum ButtonType {
    Create(Option<String>),
    Login(Option<String>),
    Save(Option<String>),
    Confirm(Option<String>),
    Cancel(Option<String>),
    Reset(Option<String>),
    Custom(Option<String>),
}

impl ButtonType {
    pub fn button_text(&self) -> String {
        match self {
            ButtonType::Create(text) => text.clone().unwrap_or_else(|| "Create".to_string()),
            ButtonType::Login(text) => text.clone().unwrap_or_else(|| "Login".to_string()),
            ButtonType::Save(text) => text.clone().unwrap_or_else(|| "Save".to_string()),
            ButtonType::Confirm(text) => text.clone().unwrap_or_else(|| "Confirm".to_string()),
            ButtonType::Cancel(text) => text.clone().unwrap_or_else(|| "Cancel".to_string()),
            ButtonType::Reset(text) => text.clone().unwrap_or_else(|| "Reset".to_string()),
            ButtonType::Custom(text) => text.clone().unwrap_or_else(|| "Custom".to_string()),
        }
    }

    pub fn button_class(&self, is_disabled: bool) -> &'static str {
        if is_disabled {
            "inline-block px-3 bg-gray-300 text-white font-bold py-2 rounded \
             cursor-not-allowed"
        } else {
            match self {
                ButtonType::Create(_) => {
                    "inline-block px-3 bg-orange-600 hover:bg-orange-700 \
                     text-white font-bold py-2 rounded"
                }
                ButtonType::Login(_) => {
                    "inline-block px-3 bg-blue-600 hover:bg-blue-700 \
                     text-white font-bold py-2 rounded"
                }
                ButtonType::Save(_) => {
                    "inline-block px-3 bg-yellow-600 hover:bg-yellow-700 \
                     text-white font-bold py-2 rounded"
                }
                ButtonType::Confirm(_) => {
                    "inline-block px-3 bg-green-600 hover:bg-green-700 \
                     text-white font-bold py-2 rounded"
                }
                ButtonType::Cancel(_) => {
                    "inline-block px-3 bg-red-600 hover:bg-red-700 \
                     text-white font-bold py-2 rounded"
                }
                ButtonType::Reset(_) => {
                    "inline-block px-3 bg-red-600 hover:bg-red-700 \
                     text-white font-bold py-2 rounded"
                }
                ButtonType::Custom(_) => {
                    "inline-block px-3 bg-purple-600 hover:bg-purple-700 \
                     text-white font-bold py-2 rounded"
                }
            }
        }
    }
}

