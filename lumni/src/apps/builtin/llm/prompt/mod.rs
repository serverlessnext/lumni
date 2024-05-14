#[cfg(feature = "cli")]
pub mod src {
    mod app;
    mod handler;
    mod session;
    mod tui;
    pub use handler::Handler;
}

#[cfg(not(feature = "cli"))]
pub mod src {
    mod handler;
    pub use handler::Handler;
}

pub use crate::external as lumni;
