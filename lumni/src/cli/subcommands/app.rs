use clap::{Arg, Command};

pub use super::app_handler::{handle_application, handle_apps};

pub fn apps_subcommand() -> Command {
    Command::new("apps").about("List available applications")
}
