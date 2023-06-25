#[allow(unused)]
#[derive(Clone)]
pub enum ButtonType {
    Create,
    Login,
    Save,
    Change,
    Confirm,
    Cancel,
    Reset,
    Submit,
}

impl ButtonType {
    fn color(&self) -> (&'static str, &'static str) {
        match self {
            // keep full color list including bg- and hover:bg- classes
            // else tailwind will not add them to css
            ButtonType::Create => ("bg-orange-600", "hover:bg-orange-700"),
            ButtonType::Login => ("bg-blue-600", "hover:bg-blue-700"),
            ButtonType::Save => ("bg-yellow-600", "hover:bg-yellow-700"),
            ButtonType::Change => ("bg-yellow-600", "hover:bg-yellow-700"),
            ButtonType::Confirm => ("bg-green-600", "hover:bg-green-700"),
            ButtonType::Cancel => ("bg-red-600", "hover:bg-red-700"),
            ButtonType::Reset => ("bg-red-600", "hover:bg-red-700"),
            ButtonType::Submit => ("bg-purple-600", "hover:bg-purple-700"),
        }
    }

    pub fn button_text(&self) -> &'static str {
        match self {
            ButtonType::Create => "Create",
            ButtonType::Login => "Login",
            ButtonType::Save => "Save",
            ButtonType::Change => "Change",
            ButtonType::Confirm => "Confirm",
            ButtonType::Cancel => "Cancel",
            ButtonType::Reset => "Reset",
            ButtonType::Submit => "Submit",
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
                "inline-block px-3 {} {} text-white font-bold py-2 rounded",
                color_normal, color_hover
            )
        }
    }
}

