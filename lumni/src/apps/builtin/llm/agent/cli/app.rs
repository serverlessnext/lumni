use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Terminal;
use textwrap::{wrap, Options, WordSplitter};
use tokio::sync::mpsc;
use tokio::time::{self, interval, Duration};
use tui_textarea::{Input, TextArea};

use super::textarea::{
    transition_command_line, CommandLine, LayoutMode, TextAreaHandler,
    TransitionAction,
};

fn draw_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    editor: &mut TextAreaHandler,
    //prompt_log: &mut TextArea,
    prompt_log: &mut PromptLog,
    command_line: &TextArea,
) -> Result<(), io::Error> {
    terminal.draw(|f| {
        let terminal_size = f.size();
        const COMMAND_LINE_HEIGHT: u16 = 3;

        let prompt_log_area;
        let prompt_edit_area;
        let prompt_log_area_scrollbar;
        let command_line_area;

        match editor.layout_mode(terminal_size) {
            LayoutMode::HorizontalSplit => {
                let response_height = 8; // minimum height for response

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(40), // max-40% space for prompt (after min space is met)
                        Constraint::Min(response_height + COMMAND_LINE_HEIGHT), // command-line
                    ])
                    .split(terminal_size);

                let bottom_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(response_height), // Apply directly as no .min() available
                        Constraint::Length(COMMAND_LINE_HEIGHT),
                    ])
                    .split(chunks[1]);

                prompt_edit_area = chunks[0];
                prompt_log_area = bottom_chunks[0];
                prompt_log_area_scrollbar = chunks[1];
                command_line_area = bottom_chunks[1];
            }
            LayoutMode::VerticalSplit => {
                // Apply vertical split logic here
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0), // container for prompt_edit and prompt_log
                        Constraint::Length(COMMAND_LINE_HEIGHT), // command line
                    ])
                    .split(terminal_size);

                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50), // left half for prompt
                        Constraint::Percentage(50), // right half for chat history
                        Constraint::Length(2),      // vertical scrollbar
                    ])
                    .split(chunks[0]);

                prompt_edit_area = main_chunks[0];
                prompt_log_area = main_chunks[1];
                prompt_log_area_scrollbar = main_chunks[2];
                command_line_area = chunks[1];
            }
        }
        f.render_widget(editor.ta_prompt_edit().widget(), prompt_edit_area);

        f.render_widget(prompt_log.widget(&prompt_log_area), prompt_log_area);
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            prompt_log_area_scrollbar,
            &mut prompt_log.vertical_scroll_state,
        );

        f.render_widget(command_line.widget(), command_line_area);
    })?;
    Ok(())
}

pub async fn run_cli(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut editor = TextAreaHandler::new();

    let mut prompt_log = PromptLog::new();

    let mut command_line = TextArea::default();
    command_line.set_cursor_line_style(Style::default());
    command_line.set_placeholder_text("Ready");

    let (tx, mut rx) = mpsc::channel(32);
    let mut tick = interval(Duration::from_millis(10));
    let mut stream_active = false;

    let is_running = Arc::new(AtomicBool::new(true));
    let mut current_mode = TransitionAction::EditPrompt;

    let mut command_line_handler = CommandLine::new();
    let mut redraw_ui = true;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if redraw_ui {
                    draw_ui(&mut terminal, &mut editor, &mut prompt_log, &command_line)?;
                    redraw_ui = false;
                }

                if poll(Duration::from_millis(50))? {
                    let event = read()?;

                    if let Event::Key(key_event) = event {
                        let input: Input = key_event.into();
                        current_mode = match current_mode {
                            TransitionAction::CommandLine => {
                                // currently in command line mode
                                match transition_command_line(
                                    &mut command_line_handler,
                                    &mut command_line,
                                    editor.ta_prompt_edit(),
                                    input.clone()
                                ).await {
                                    TransitionAction::Quit => {
                                        break; // Exit the loop immediately if Quit is returned
                                    },
                                    TransitionAction::EditPrompt => TransitionAction::EditPrompt,
                                    TransitionAction::WritePrompt(prompt) => {
                                        // Initiate streaming if not already active
                                        if !stream_active {
                                            is_running.store(true, Ordering::SeqCst);
                                            start_streaming(tx.clone(), is_running.clone(), prompt).await;
                                            stream_active = true;
                                        }
                                        TransitionAction::EditPrompt     // Switch to editor mode
                                    },
                                    _ => TransitionAction::CommandLine, // Stay in command line mode
                                }
                            },
                            _ => {
                                // editor mode
                                match editor.transition(&input).await {
                                    TransitionAction::Quit => {
                                        break; // Exit the loop immediately if Quit is returned
                                    },
                                    TransitionAction::CommandLine => {
                                        command_line.insert_str(":");
                                        stream_active = false; // Stop streaming if command line is activated
                                        is_running.store(false, Ordering::SeqCst); //reset
                                        TransitionAction::CommandLine
                                    },
                                    _ => TransitionAction::EditPrompt,
                                }
                            },
                        };
                    }
                    redraw_ui = true;   // redraw the UI after each type of event
                }
            },
            Some(response) = rx.recv() => {
                prompt_log.insert_str(&format!("{}", response));
                redraw_ui = true;
            },
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn start_streaming(
    tx: mpsc::Sender<String>,
    is_running: Arc<AtomicBool>,
    prompt: String,
) {
    tokio::spawn(async move {
        // First, send the initial formatted question
        let initial_question = format!("Q: {}\nBot:", prompt);
        if tx.send(initial_question).await.is_err() {
            println!("Receiver dropped");
            return;
        }
        time::sleep(Duration::from_millis(100)).await; // Simulate a slight delay after the question

        // Words to simulate a bot's streaming response
        let response_words = vec![
            "some", "random", "answer", "to", "simulate", "a", "streaming", "response", "from", "a", "bot",
        ];

        // Stream each word one by one
        for word in response_words {
            if !is_running.load(Ordering::SeqCst) {
                break; // Stop sending if is_running is set to false
            }
            let mut response_text = String::from("");
            response_text.push(' '); // Add a space before each word
            response_text.push_str(word); // Append the word to the ongoing sentence

            if tx.send(response_text.clone()).await.is_err() {
                println!("Receiver dropped");
                return;
            }
            time::sleep(Duration::from_millis(50)).await; // Simulate time between sending each word
        }

        // Reset is_running after completion
        is_running.store(false, Ordering::SeqCst);
    });
}

pub struct PromptLog {
    text: String,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl PromptLog {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
        }
    }

    /// Updates the scroll state based on the current size of the terminal.
    pub fn update_scroll_state(&mut self, size: &Rect) {
        let height = (size.height - 2) as usize;
        let lines = self.process_text(&size).len();
        self.vertical_scroll = if lines > height { lines - height } else { 0 };

        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(self.vertical_scroll)
            .viewport_content_length(height)
            .position(self.vertical_scroll + height);
    }

    fn process_text(&self, size: &Rect) -> Vec<Line> {
        self.text
            .split('\n')
            .flat_map(|line| {
                wrap(
                    line,
                    Options::new((size.width - 2) as usize)
                        .word_splitter(WordSplitter::NoHyphenation),
                )
                .into_iter()
                .map(|cow_str| Line::from(Span::from(cow_str.to_string())))
                .collect::<Vec<Line>>()
            })
            .collect()
    }

    pub fn widget(&mut self, size: &Rect) -> Paragraph {
        self.update_scroll_state(size);

        let text_lines = self.process_text(&size);
        //eprintln!("Text lines: {:?}", text_lines.len());
        Paragraph::new(Text::from(text_lines))
            .block(Block::default().title("Paragraph").borders(Borders::ALL))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            //.wrap(Wrap { trim: true })
            .scroll((self.vertical_scroll as u16, 0))
    }

    pub fn insert_str(&mut self, text: &str) {
        self.text.push_str(text);
    }
}
