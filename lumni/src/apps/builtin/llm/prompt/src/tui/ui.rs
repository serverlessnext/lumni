use super::{
    CommandLine, ModalConfigWindow, ModalWindowTrait, ModalWindowType,
    PromptWindow, ResponseWindow, TextWindowTrait, WindowEvent,
    WindowKind,
};

pub struct TabUi<'a> {
    pub prompt: PromptWindow<'a>,
    pub response: ResponseWindow<'a>,
    pub command_line: CommandLine<'a>,
    pub primary_window: WindowKind,
    pub modal: Option<Box<dyn ModalWindowTrait>>,
}

impl TabUi<'_> {
    pub fn new() -> Self {
        Self {
            prompt: PromptWindow::new(),
            response: ResponseWindow::new(),
            command_line: CommandLine::new(),
            primary_window: WindowKind::ResponseWindow,
            modal: None,
        }
    }

    pub fn init(&mut self) {
        self.response.init(); //set_status_normal(); // initialize in normal mode
        self.prompt.set_status_normal(); // initialize with defaults
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

    pub fn set_primary_window(&mut self, window_type: WindowKind) {
        self.primary_window = match window_type {
            WindowKind::ResponseWindow | WindowKind::PromptWindow => window_type,
            _ => {
                // only ResponseWindow and PromptWindow can be primary windows
                unreachable!("Invalid primary window type: {:?}", window_type)
            }
        };
    }

    pub fn set_response_window(&mut self) -> WindowEvent {
        self.prompt.set_status_background();
        self.response.set_status_normal();
        return WindowEvent::ResponseWindow;
    }

    pub fn set_prompt_window(&mut self, insert_mode: bool) -> WindowEvent {
        self.response.set_status_background();
        if insert_mode {
            self.prompt.set_status_insert();
        } else {
            self.prompt.set_status_normal();
        }
        return WindowEvent::PromptWindow;
    }

    pub fn clear_modal(&mut self) {
        self.modal = None;
    }
}
