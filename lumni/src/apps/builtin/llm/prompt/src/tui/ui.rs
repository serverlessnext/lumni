use super::{CommandLine, PromptWindow, ResponseWindow, ContainerWindow, TextWindowTrait};

pub struct AppUi<'a> {
    pub prompt: PromptWindow<'a>,
    pub response: ResponseWindow<'a>,
    pub command_line: CommandLine<'a>,
    pub configuration: ContainerWindow,
    show_config: bool,
}

impl AppUi<'_> {
    pub fn new() -> Self {
        Self {
            prompt: PromptWindow::new(),
            response: ResponseWindow::new(),
            command_line: CommandLine::new(),
            configuration: ContainerWindow::default(),
            show_config: false,
        }
    }

    pub fn init(&mut self) {
        self.prompt.set_normal_mode(); // initialize in normal mode
        self.response.init(); // initialize with defaults
        self.command_line.init(); // initialize with defaults
    }

    pub fn get_show_config(&self) -> bool {
        self.show_config
    }

    pub fn set_show_config(&mut self, show_config: bool) {
        self.show_config = show_config;
    }
}
