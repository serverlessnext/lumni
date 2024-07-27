use clap::{Arg, ArgAction, Command};

pub use super::ls_handler::handle_ls;

pub fn ls_subcommand() -> Command {
    Command::new("ls")
        .about("List objects on Local Filesystem or an S3 bucket")
        .arg(
            Arg::new("uri")
                .index(1)
                .default_value(".") // default to current directory
                .help("URI to list objects from. E.g. s3://bucket-name/"),
        )
        .arg(
            Arg::new("name").long("name").short('n').help(
                "Filter objects based on name. E.g. 'foo', 'foo.*', '.*bar'",
            ),
        )
        .arg(
            Arg::new("size")
                .long("size")
                .short('s')
                .num_args(1)
                .allow_hyphen_values(true)
                .help(
                    "Filter objects based on size. E.g. '-1K', '+4M', '+1G', \
                     '-1G', '5G', '1G-2G'",
                ),
        )
        .arg(
            Arg::new("mtime")
                .long("mtime")
                .short('t')
                .num_args(1)
                .allow_hyphen_values(true)
                .help(
                    "Filter objects based on the time offset. E.g. '-60s', \
                     '+5m', '-1h', '+2D', '-3W', '+1M', '-1Y'",
                ),
        )
        .arg(
            Arg::new("show_hidden")
                .long("show-hidden")
                .short('H')
                .action(ArgAction::SetTrue)
                .help("Show hidden files [default: false]"),
        )
        .arg(
            Arg::new("recursive")
                .long("recursive")
                .short('r')
                .action(ArgAction::SetTrue)
                .help(
                    "List (virtual) subdirectories recursively [default: \
                     false]",
                ),
        )
        .arg(
            Arg::new("max_files")
                .long("max-files")
                .short('m')
                .default_value("1000")
                .help("Maximum number of files to list"),
        )
        .arg(
            Arg::new("ignore_file")
                .long("ignore-file")
                .short('i')
                .num_args(0..=1)
                .default_missing_value(".gitignore")
                .help(
                    "Ignore files matching patterns in the specified file. \
                     Defaults to '.gitignore' if no file is specified.",
                ),
        )
}
