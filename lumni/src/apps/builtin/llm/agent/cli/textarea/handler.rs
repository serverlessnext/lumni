use ratatui::layout::Rect;
use ratatui::style::Style;
use tui_textarea::{CursorMove, Input, Key, TextArea};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

use super::clipboard::ClipboardProvider;
use super::mode::EditorMode;

use super::{PromptLog, run_prompt};

enum TextAreaAction {
    Cut,
    Copy,
}
pub enum LayoutMode {
    HorizontalSplit,
    VerticalSplit,
}
pub struct TextAreaHandler {
    mode: EditorMode,
    last_key: Option<Key>,
    numeric_input: Option<String>,
    clipboard_provider: ClipboardProvider,
    layout_mode: LayoutMode,
    command_line_handler: Option<CommandLine>,
    ta_prompt_edit: TextArea<'static>,
    ta_prompt_log: TextArea<'static>,
    ta_command_line: TextArea<'static>,
}

impl TextAreaHandler {
    pub fn new() -> Self {
        let mut ta_prompt_log = TextArea::default();
        ta_prompt_log.set_block(EditorMode::ReadOnly.block());
        ta_prompt_log.set_cursor_style(EditorMode::ReadOnly.cursor_style());
        ta_prompt_log.set_placeholder_text("Ready");


        let mut ta_prompt_edit = TextArea::default();
        ta_prompt_edit.set_block(EditorMode::Normal.block());
        ta_prompt_edit.set_cursor_style(EditorMode::Normal.cursor_style());

        let mut ta_command_line = TextArea::default();
        ta_command_line.set_cursor_line_style(Style::default());
        ta_command_line.set_placeholder_text("Ready");

        TextAreaHandler {
            mode: EditorMode::Normal,
            last_key: None,
            numeric_input: None,
            clipboard_provider: ClipboardProvider::new(),
            layout_mode: LayoutMode::HorizontalSplit,
            command_line_handler: None,
            ta_prompt_edit,
            ta_prompt_log,
            ta_command_line,
        }
    }

    pub fn ta_prompt_edit(&mut self) -> &mut TextArea<'static> {
        &mut self.ta_prompt_edit
    }

    pub fn ta_prompt_log(&mut self) -> &mut TextArea<'static> {
        &mut self.ta_prompt_log
    }

    pub fn ta_command_line(&mut self) -> &mut TextArea<'static> {
        &mut self.ta_command_line
    }

    pub fn enable_command_line(&mut self, enable: bool) {
        if enable {
            self.command_line_handler = Some(CommandLine::new());
        } else {
            self.command_line_handler = None;
        }
    }

    pub fn layout_mode(&self, terminal_size: Rect) -> &LayoutMode {
        let rows = terminal_size.height;
        let cols = terminal_size.width;
        if rows < 20 {
            return &LayoutMode::VerticalSplit;
        }
        if cols < 120 || cols < 2 * rows {
            return &LayoutMode::HorizontalSplit;
        }
        return &LayoutMode::VerticalSplit;
        //&self.layout_mode
    }

    fn reset(&mut self) {
        self.mode = EditorMode::Normal;
        self.last_key = None;
        self.numeric_input = None;
    }

    fn handle_position_keys(&mut self, input: &Input) -> bool {
        match input.key {
            Key::Left => {
                self.ta_prompt_edit.move_cursor(CursorMove::Back);
                true
            }
            Key::Right => {
                self.ta_prompt_edit.move_cursor(CursorMove::Forward);
                true
            }
            Key::Up => {
                self.ta_prompt_edit.move_cursor(CursorMove::Up);
                true
            }
            Key::Down => {
                self.ta_prompt_edit.move_cursor(CursorMove::Down);
                true
            }
            Key::Home => {
                self.ta_prompt_edit.move_cursor(CursorMove::Head);
                true
            }
            Key::End => {
                self.ta_prompt_edit.move_cursor(CursorMove::End);
                true
            }
            Key::PageUp => {
                // Implement according to your logic
                false
            }
            Key::PageDown => {
                // Implement according to your logic
                false
            }
            _ => false,
        }
    }

    fn update_numeric_input(&mut self, c: char) {
        if self.numeric_input.is_none() && c == '0' {
            // Ignore leading zeros
            return;
        }
        match &self.numeric_input {
            Some(n) => {
                let new_input = n.to_string() + &c.to_string();
                self.numeric_input = Some(new_input);
            }
            None => {
                self.numeric_input = Some(c.to_string());
            }
        }
    }

    fn set_vim_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
        self.ta_prompt_edit.set_block(mode.block());
        self.ta_prompt_edit.set_cursor_style(mode.cursor_style());
    }

    pub fn yank_to_clipboard(&mut self) {
        self.ta_prompt_edit.cancel_selection();
        if let Err(e) = self
            .clipboard_provider
            .write_line(&self.ta_prompt_edit.yank_text(), false)
        {
            eprintln!("Clipboard error: {}", e);
        }
    }

    pub fn paste_from_clipboard(&mut self) {
        if let Ok(text) = self.clipboard_provider.read_text() {
            self.ta_prompt_edit.set_yank_text(&text); // copy from clipboard

            if text.contains("\n") {
                // go to end of line
                self.ta_prompt_edit.move_cursor(CursorMove::End);
                self.ta_prompt_edit.insert_newline();
                self.ta_prompt_edit.paste();
                self.ta_prompt_edit.move_cursor(CursorMove::Up);
            } else {
                self.ta_prompt_edit.paste();
            }
        }
    }

    fn select_lines(&mut self, action: TextAreaAction) {
        // Start the selection at the head of the current line
        self.ta_prompt_edit.move_cursor(CursorMove::Head);
        self.ta_prompt_edit.start_selection();

        // Determine the number of lines to select based on numeric_input
        let lines_to_select = self
            .numeric_input
            .as_ref()
            .map_or(1, |n| n.parse::<usize>().unwrap_or(1));

        // Move down (lines_to_select - 1) times to select the correct number of lines
        // This adjustment ensures we do not select an extra line
        for _ in 0..(lines_to_select - 1) {
            self.ta_prompt_edit.move_cursor(CursorMove::Down);
        }
        // Extend the selection to the end of the last intended line
        self.ta_prompt_edit.move_cursor(CursorMove::End);

        match action {
            TextAreaAction::Cut => {
                self.ta_prompt_edit.cut();
            }
            TextAreaAction::Copy => {
                self.ta_prompt_edit.copy();
            }
        }

        self.yank_to_clipboard();

        self.last_key = None;
        self.numeric_input = None;
    }

    // Handle input and transition between modes
    pub async fn transition(&mut self, input: &Input) -> bool {
        if let Some(cl) = &mut self.command_line_handler {
            // handle command line mode
            match input {
                Input { key: Key::Esc, .. } => {
                    // catch esc key - clear command line
                    cl.clear(&mut self.ta_command_line)
                }
                Input {
                    key: Key::Enter, ..
                } => {
                    // catch enter key - discontinues cl mode
                    let status = cl
                        .process_command(
                            &mut self.ta_command_line,
                            &self.ta_prompt_edit,
                            &mut self.ta_prompt_log,
                        )
                        .await;
                    if status {
                        return true; // quit application
                    }
                }
                _ => {
                    self.ta_command_line.input(input.clone());
                    return false; // continue in command line mode
                }
            };
            // exit command line mode
            self.enable_command_line(false);
        }

        // handle position keys for non insert mode
        if self.mode != EditorMode::Insert && self.handle_position_keys(input) {
            return false;
        }

        match self.mode {
            EditorMode::Normal => match input {
                Input { key: Key::Esc, .. } => {
                    self.reset();
                }
                Input {
                    key: Key::Char('i'),
                    ..
                } => {
                    self.set_vim_mode(EditorMode::Insert);
                }
                Input {
                    key: Key::Char('v'),
                    ..
                } => {
                    self.set_vim_mode(EditorMode::Visual);
                    self.ta_prompt_edit.start_selection(); // Start selection
                }
                // Delete character before cursor
                Input {
                    key: Key::Char('x'),
                    ..
                } => {
                    self.ta_prompt_edit.delete_next_char();
                }
                // :
                Input {
                    key: Key::Char(':'),
                    ..
                } => {
                    self.ta_command_line.insert_str(":");
                    self.enable_command_line(true);
                }
                Input {
                    key: Key::Char('$'),
                    ..
                } => {
                    self.ta_prompt_edit.move_cursor(CursorMove::End);
                }
                Input {
                    key: Key::Char('G'),
                    ..
                } => {
                    self.ta_prompt_edit.move_cursor(CursorMove::Bottom);
                    self.ta_prompt_edit.move_cursor(CursorMove::End);
                }
                Input {
                    key: Key::Char('g'),
                    ..
                } => {
                    if let Some(Key::Char('g')) = self.last_key {
                        self.ta_prompt_edit.move_cursor(CursorMove::Top);
                        self.ta_prompt_edit.move_cursor(CursorMove::Head);
                    } else {
                        // Record the first "g" press
                        self.last_key = Some(Key::Char('g'));
                    }
                }
                // "yy" to yank lines
                Input {
                    key: Key::Char('y'),
                    ..
                } => {
                    if let Some(Key::Char('y')) = self.last_key {
                        self.select_lines(TextAreaAction::Copy);
                    } else {
                        // Record the first "y" press
                        self.last_key = Some(Key::Char('y'));
                    }
                }
                Input {
                    key: Key::Char('d'),
                    ..
                } => {
                    if let Some(Key::Char('d')) = self.last_key {
                        self.select_lines(TextAreaAction::Cut);
                    } else {
                        // Record the first "d" press
                        self.last_key = Some(Key::Char('d'));
                    }
                }
                Input {
                    key: Key::Delete, ..
                } => {
                    self.ta_prompt_edit.delete_next_char();
                }
                Input {
                    key: Key::Backspace,
                    ..
                } => {
                    self.ta_prompt_edit.delete_char();
                }
                // Move cursor
                Input {
                    key: Key::Char('h'),
                    ..
                } => self.ta_prompt_edit.move_cursor(CursorMove::Back),
                Input {
                    key: Key::Char('l'),
                    ..
                } => self.ta_prompt_edit.move_cursor(CursorMove::Forward),
                Input {
                    key: Key::Char('k'),
                    ..
                } => {
                    let lines_to_move = self
                        .numeric_input
                        .as_ref()
                        .map_or(1, |n| n.parse::<usize>().unwrap_or(1));
                    for _ in 0..lines_to_move {
                        self.ta_prompt_edit.move_cursor(CursorMove::Up);
                    }
                    self.numeric_input = None;
                }
                Input {
                    key: Key::Char('j'),
                    ..
                } => {
                    let lines_to_move = self
                        .numeric_input
                        .as_ref()
                        .map_or(1, |n| n.parse::<usize>().unwrap_or(1));
                    for _ in 0..lines_to_move {
                        self.ta_prompt_edit.move_cursor(CursorMove::Down);
                    }
                    self.numeric_input = None;
                }
                // Undo/Redo
                Input {
                    key: Key::Char('u'),
                    ctrl: false,
                    ..
                } => {
                    self.ta_prompt_edit.undo();
                }
                Input {
                    key: Key::Char('r'),
                    ctrl: true,
                    ..
                } => {
                    self.ta_prompt_edit.redo();
                }
                // Paste yanked text
                Input {
                    key: Key::Char('p'),
                    ctrl: false,
                    ..
                } => {
                    self.paste_from_clipboard();
                }
                Input {
                    key: Key::Char('0'..='9'),
                    ..
                } => {
                    // if numeric_input is None, move to beginning of line
                    if self.numeric_input.is_none()
                        && input.key == Key::Char('0')
                    {
                        self.ta_prompt_edit.move_cursor(CursorMove::Head);
                    } else {
                        match input.key {
                            Key::Char(c) => {
                                self.update_numeric_input(c);
                            }
                            _ => {}
                        }
                    }
                }
                // Change layout (shift + t)
                Input {
                    key: Key::Char('T'),
                    ..
                } => {
                    self.layout_mode = match self.layout_mode {
                        LayoutMode::HorizontalSplit => {
                            LayoutMode::VerticalSplit
                        }
                        LayoutMode::VerticalSplit => {
                            LayoutMode::HorizontalSplit
                        }
                    };
                }
                _ => {} // Ignore other keys in Normal mode
            },
            EditorMode::Insert => match input {
                Input { key: Key::Esc, .. } => {
                    self.ta_prompt_edit.cancel_selection();
                    self.set_vim_mode(EditorMode::Normal);
                }
                Input {
                    key: Key::Char('v'),
                    ctrl: true,
                    ..
                } => {
                    self.paste_from_clipboard();
                }
                _ => {
                    // In Insert mode, most keys should result in text input.
                    // Pass the input to the textarea in your main loop, not here.
                    self.ta_prompt_edit.input(input.clone());
                }
            },
            EditorMode::Visual => match input {
                Input { key: Key::Esc, .. } => {
                    self.ta_prompt_edit.cancel_selection();
                    self.set_vim_mode(EditorMode::Normal);
                }
                // tab 
                Input { key: Key::Tab, .. } => {
                    eprintln!("Tab hit");
                    //self.ta_prompt_edit.indent();
                }

                Input {
                    key: Key::Char('$'),
                    ..
                } => {
                    self.ta_prompt_edit.move_cursor(CursorMove::End);
                }
                Input {
                    key: Key::Char('g'),
                    ..
                } => {
                    if let Some(Key::Char('g')) = self.last_key {
                        self.ta_prompt_edit.move_cursor(CursorMove::Top);
                        self.ta_prompt_edit.move_cursor(CursorMove::Head);
                    } else {
                        // Record the first "g" press
                        self.last_key = Some(Key::Char('g'));
                    }
                }
                Input {
                    key: Key::Char('k'),
                    ..
                } => {
                    let lines_to_move = self
                        .numeric_input
                        .as_ref()
                        .map_or(1, |n| n.parse::<usize>().unwrap_or(1));
                    for _ in 0..lines_to_move {
                        self.ta_prompt_edit.move_cursor(CursorMove::Up);
                    }
                    self.numeric_input = None;
                }
                Input {
                    key: Key::Char('j'),
                    ..
                } => {
                    let lines_to_move = self
                        .numeric_input
                        .as_ref()
                        .map_or(1, |n| n.parse::<usize>().unwrap_or(1));
                    for _ in 0..lines_to_move {
                        self.ta_prompt_edit.move_cursor(CursorMove::Down);
                    }
                    self.numeric_input = None;
                }
                Input {
                    key: Key::Char('G'),
                    ..
                } => {
                    self.ta_prompt_edit.move_cursor(CursorMove::Bottom);
                    self.ta_prompt_edit.move_cursor(CursorMove::End);
                }
                Input {
                    key: Key::Char('y'),
                    ctrl: false,
                    ..
                } => {
                    self.ta_prompt_edit.copy();
                    self.yank_to_clipboard();
                    self.set_vim_mode(EditorMode::Normal);
                }
                Input {
                    key: Key::Char('0'..='9'),
                    ..
                } => {
                    // if numeric_input is None, move to beginning of line
                    if self.numeric_input.is_none()
                        && input.key == Key::Char('0')
                    {
                        self.ta_prompt_edit.move_cursor(CursorMove::Head);
                    } else {
                        match input.key {
                            Key::Char(c) => {
                                self.update_numeric_input(c);
                            }
                            _ => {}
                        }
                    }
                }
                // (c)ut, (d)elete, (x) delete behave the same in visual mode
                Input {
                    key: Key::Char(c),
                    ctrl: false,
                    ..
                } if *c == 'c' || *c == 'x' || *c == 'd' => {
                    self.ta_prompt_edit.cut();
                    self.yank_to_clipboard();
                    self.set_vim_mode(EditorMode::Normal);
                }
                Input {
                    key: Key::Char('p'),
                    ctrl: false,
                    ..
                } => {
                    self.ta_prompt_edit.paste();
                }
                _ => {} // Ignore other keys in Visual mode
            },
            EditorMode::ReadOnly => {
                // used for Log view 
                // similar to Visual, except no Cut, Delete, or Paste
                // requires Tab to switch back to 

            }
        }
        false // Do not quit unless specified
    }
}

pub struct CommandLine {
    prompt_log: PromptLog,
}

impl CommandLine {
    pub fn new() -> Self {
        let instruction = "A chat between a curious human and an artificial intelligence assistant. The assistant gives helpful, detailed, and polite answers to the human's questions.".to_string();
        let prompt_log = PromptLog::new(10, instruction);
        Self { prompt_log }
    }

    pub fn clear(&mut self, command_line: &mut TextArea<'_>) {
        command_line.select_all();
        command_line.cut();
    }

    pub async fn process_command(
        &mut self,
        command_line: &mut TextArea<'_>,
        ta_prompt_edit: &TextArea<'_>,
        ta_prompt_log: &mut TextArea<'_>,
    ) -> bool {
        let command = command_line.lines()[0].to_string();
        self.clear(command_line);

        if command.starts_with(':') {
            match command.trim_start_matches(':') {
                "q" => return true, // Quit by returning true
                "w" => {
                    let question = ta_prompt_edit.lines().join("\n");
                    // Initiate the streaming response process
                    let mut response_stream = self.run_prompt(&question).await;

                    while let Some(response) = response_stream.recv().await {
                        //eprintln!("Response: {}", response);
                        ta_prompt_log.insert_str(&format!("{}\n", response));
                        ta_prompt_log.widget();
                    }
                },
                _ => {}, // Handle other commands as needed
            }
        }
        false // continue running
    }

    async fn run_prompt(&mut self, _question: &str) -> mpsc::Receiver<String> {
        // simulated version to test streaming response
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let responses = vec![
                "Processing your request...",
                "Fetching data...",
                "The answer is 42.",
                "Is there anything else you'd like to know?",
            ];

            for response in responses {
                if tx.send(response.to_string()).await.is_err() {
                    eprintln!("Receiver dropped");
                    return;
                }
                // Simulate delay for streaming
                time::sleep(Duration::from_secs(1)).await;
            }
        });

        rx // Return the receiver to the caller
    }
}
