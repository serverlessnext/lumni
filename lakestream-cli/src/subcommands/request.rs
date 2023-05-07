use clap::{Arg, Command};

pub use super::request_handler::handle_request;

pub fn request_subcommand() -> Command {
    Command::new("-X")
        .long_flag("request")
        .about("Performs an HTTP request")
        .arg(
            Arg::new("method")
                .index(1)
                .value_parser(["GET"])
                .required(true)
                .help("HTTP verb for the request (GET)"),
        )
        .arg(
            Arg::new("uri")
                .index(2)
                .required(true)
                .help("File for the HTTP request"),
        )
}
