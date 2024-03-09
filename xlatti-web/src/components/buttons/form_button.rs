use leptos::*;

use super::button_type::ButtonType;

#[derive(Clone)]
pub struct FormButton {
    button_type: ButtonType,
    enabled: bool,
    text: Option<String>,
}

impl FormButton {
    pub fn new(button_type: ButtonType, text: Option<&str>) -> Self {
        Self {
            button_type,
            enabled: true, // default
            text: text.map(|s| s.to_string()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn text(&self) -> String {
        self.text
            .clone()
            .unwrap_or_else(|| self.button_type.button_text().to_string())
    }

    pub fn button_class(&self) -> String {
        self.button_type.button_class(!self.is_enabled())
    }

    pub fn into_view(self) -> impl IntoView {
        view! {
            <button
                type="submit"
                class=self.button_class()
                disabled={!self.is_enabled()}
            >
                {self.text()}
            </button>
        }
        .into_view()
    }
}

#[derive(Clone)]
pub struct FormButtonGroup {
    buttons: Vec<FormButton>,
    // true = enable, false = disable, None = no action
    enable_on_change: Option<bool>,
}

impl FormButtonGroup {
    pub fn new(enable_on_change: Option<bool>) -> Self {
        Self {
            buttons: Vec::new(),
            enable_on_change,
        }
    }

    pub fn add_button(&mut self, button: FormButton) {
        self.buttons.push(button);
    }

    pub fn into_view(self, form_change: Option<bool>) -> impl IntoView {
        let enable_on_change = self.enable_on_change.unwrap_or(false);
        let buttons = self.buttons;

        view! {
            <For
                each=move || buttons.clone().into_iter().enumerate()
                key=|(index, _)| *index
                children=move | (_, button)| {
                    if form_change.unwrap_or(false) && enable_on_change {
                        button.set_enabled(enable_on_change).into_view()
                    } else {
                        button.into_view()
                    }
                }
            />
        }
        .into_view()
    }
}
