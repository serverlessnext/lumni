use clap::{Arg, Command};

pub use super::app_handler::handle_app;


pub fn app_subcommand() -> Command {
    Command::new("app")
        .about("Run an application")
}
