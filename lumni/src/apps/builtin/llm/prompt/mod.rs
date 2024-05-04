mod handler;
pub use handler::Handler;


#[cfg(feature = "cli")]
mod cli {
    mod app;
    mod prompt;
    mod tui;
    pub use app::run_cli;
}
