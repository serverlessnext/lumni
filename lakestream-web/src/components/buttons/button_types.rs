#[allow(unused)]
#[derive(Clone)]
pub enum ButtonType {
    Create(Option<String>),
    Login(Option<String>),
    Save(Option<String>),
    Change(Option<String>),
    Confirm(Option<String>),
    Cancel(Option<String>),
    Reset(Option<String>),
    Custom(Option<String>),
}

impl ButtonType {
    fn color(&self) -> (&'static str, &'static str) {
        match self {
            ButtonType::Create(_) => ("orange-600", "orange-700"),
            ButtonType::Login(_) => ("blue-600", "blue-700"),
            ButtonType::Save(_) => ("yellow-600", "yellow-700"),
            ButtonType::Change(_) => ("yellow-600", "yellow-700"),
            ButtonType::Confirm(_) => ("green-600", "green-700"),
            ButtonType::Cancel(_) => ("red-600", "red-700"),
            ButtonType::Reset(_) => ("red-600", "red-700"),
            ButtonType::Custom(_) => ("purple-600", "purple-700"),
        }
    }

    pub fn button_text(&self) -> String {
        match self {
            ButtonType::Create(text) => {
                text.clone().unwrap_or_else(|| "Create".to_string())
            }
            ButtonType::Login(text) => {
                text.clone().unwrap_or_else(|| "Login".to_string())
            }
            ButtonType::Save(text) => {
                text.clone().unwrap_or_else(|| "Save".to_string())
            }
            ButtonType::Change(text) => {
                text.clone().unwrap_or_else(|| "Change".to_string())
            }
            ButtonType::Confirm(text) => {
                text.clone().unwrap_or_else(|| "Confirm".to_string())
            }
            ButtonType::Cancel(text) => {
                text.clone().unwrap_or_else(|| "Cancel".to_string())
            }
            ButtonType::Reset(text) => {
                text.clone().unwrap_or_else(|| "Reset".to_string())
            }
            ButtonType::Custom(text) => {
                text.clone().unwrap_or_else(|| "Custom".to_string())
            }
        }
    }

    pub fn button_class(&self, is_disabled: bool) -> String {
        let (color_normal, color_hover) = self.color();
        if is_disabled {
            "inline-block px-3 bg-gray-300 text-white font-bold py-2 rounded \
             cursor-not-allowed"
                .to_string()
        } else {
            format!(
                "inline-block px-3 bg-{} hover:bg-{} text-white font-bold \
                 py-2 rounded",
                color_normal, color_hover
            )
        }
    }
}
