use clap::{Arg, Command};

pub use super::request_handler::handle_request;

pub fn request_subcommand() -> Command {
    Command::new("-X")
        .long_flag("request")
        .about("Executes an HTTP request")
        .allow_missing_positional(true)
        .arg_required_else_help(true)
        .after_help("Use -X/--request [GET,HEAD,POST] [URI]")
        .arg(
            Arg::new("method")
                .default_value("GET")
                .required(false)
                .index(1)
                .value_parser(["GET", "POST"])
                .help("HTTP verb for the request (e.g. GET, POST)"),
        )
        .arg(
            Arg::new("uri")
                .index(2)
                .required(true)
                .help("URI for the HTTP request"),
        )
}
