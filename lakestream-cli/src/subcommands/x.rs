
pub use super::x_handler::handle_x;

use clap::{Arg, Command};


pub fn x_subcommand() -> Command {
    Command::new("-X")
        .about("Performs an HTTP request (eg. GET or PUT)")
        .arg(
            Arg::new("method")
                .index(1)
                .value_parser(["GET", "PUT"])
                .required(true)
                .help("HTTP verb for the request (GET or PUT)"),
        )
        .arg(
            Arg::new("uri")
                .index(2)
                .required(false)
                .help("File for the HTTP request"),
        )
}
