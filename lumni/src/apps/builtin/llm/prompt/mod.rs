#[cfg(feature = "cli")]
pub mod src {
    mod app;
    mod chat;
    mod defaults;
    mod handler;
    mod model;
    mod server;
    mod tui;
    pub use handler::Handler;
}

#[cfg(not(feature = "cli"))]
pub mod src {
    mod handler;
    pub use handler::Handler;
}
#[allow(unused_imports)]
pub use crate::external as lumni;
