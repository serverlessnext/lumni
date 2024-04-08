mod handler;
pub use handler::Handler;

#[cfg(feature = "cli")]
pub mod cli {
    pub mod prompter;
    pub mod textarea;
    pub use prompter::run_prompter;
}
