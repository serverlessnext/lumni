use super::{
    CommandLine, ModalWindow, PromptWindow, ResponseWindow, TextWindowTrait,
};

pub struct AppUi<'a> {
    pub prompt: PromptWindow<'a>,
    pub response: ResponseWindow<'a>,
    pub command_line: CommandLine<'a>,
    pub modal: Option<ModalWindow>,
}

impl AppUi<'_> {
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

    pub fn set_modal(&mut self, modal: ModalWindow) {
        self.modal = Some(modal);
    }

    pub fn clear_modal(&mut self) {
        self.modal = None;
    }
}
