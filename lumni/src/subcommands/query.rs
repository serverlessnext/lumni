use clap::{Arg, Command};

pub use super::query_handler::handle_query;

pub fn query_subcommand() -> Command {
    Command::new("-Q")
        .long_flag("query")
        .about("Executes a Query")
        .arg(
            Arg::new("statement")
                .index(1)
                .required(true)
                .help("Query statement"),
        )
}
