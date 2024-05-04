
#[cfg(feature = "cli")]
pub mod src {
    mod app;
    mod prompt;
    mod tui;
    mod handler;
    pub use handler::Handler;
}

#[cfg(not(feature = "cli"))]
pub mod src {
    mod handler;
    pub use handler::Handler;
}