use clap::{Arg, Command};

use super::ls::ls_shared_args;
pub use super::query_handler::handle_query;

pub fn query_subcommand() -> Command {
    let mut query = Command::new("-Q")
        .long_flag("query")
        .about("Executes a Query")
        .arg(
            Arg::new("statement")
                .index(1)
                .required(true)
                .help("Query statement"),
        );

    for arg in ls_shared_args() {
        query = query.arg(arg);
    }
    query
}
