use super::{
    CommandLine, ModalConfigWindow, ModalWindowTrait, ModalWindowType,
    PromptWindow, ResponseWindow, TextWindowTrait,
};

pub struct TabUi<'a> {
    pub prompt: PromptWindow<'a>,
    pub response: ResponseWindow<'a>,
    pub command_line: CommandLine<'a>,
    pub modal: Option<Box<dyn ModalWindowTrait>>,
}

impl TabUi<'_> {
    pub fn new() -> Self {
        Self {
            prompt: PromptWindow::new(),
            response: ResponseWindow::new(),
            command_line: CommandLine::new(),
            modal: None,
        }
    }

    pub fn init(&mut self) {
        self.prompt.set_normal_mode(); // initialize in normal mode
        self.response.init(); // initialize with defaults
        self.command_line.init(); // initialize with defaults
    }

    pub fn set_new_modal(&mut self, modal_type: ModalWindowType) {
        self.modal = match modal_type {
            ModalWindowType::Config => Some(Box::new(ModalConfigWindow::new())),
        };
    }

    pub fn needs_modal_update(&self, new_type: ModalWindowType) -> bool {
        match self.modal.as_ref() {
            Some(modal) => new_type != modal.get_type(),
            None => true,
        }
    }

    pub fn clear_modal(&mut self) {
        self.modal = None;
    }
}
