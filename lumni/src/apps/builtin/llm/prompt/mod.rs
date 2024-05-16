#[cfg(feature = "cli")]
pub mod src {
    mod app;
    mod chat;
    mod handler;
    mod tui;
    pub use handler::Handler;
}

#[cfg(not(feature = "cli"))]
pub mod src {
    mod handler;
    pub use handler::Handler;
}

pub use crate::external as lumni;
