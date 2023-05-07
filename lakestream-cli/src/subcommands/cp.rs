use clap::{Arg, Command};

pub use super::cp_handler::handle_cp;

pub fn cp_subcommand() -> Command {
    Command::new("cp")
        .about("Copy objects between source and target URIs")
        .arg(
            Arg::new("source")
                .index(1)
                .required(true)
                .help("Source URI to copy objects from"),
        )
        .arg(
            Arg::new("target")
                .index(2)
                .required(true)
                .help("Target URI to copy objects to"),
        )
}
