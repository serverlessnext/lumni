use std::env;

use clap::{Arg, ArgAction, Command};

use super::ls_command::handle_ls;

const PROGRAM_NAME: &str = "lakestream";

pub fn run_cli(args: Vec<String>) {
    let app =
        Command::new(PROGRAM_NAME)
            .version(env!("CARGO_PKG_VERSION"))
            .arg_required_else_help(true)
            .about(format!(
                "List objects in an S3 bucket\n\nExample:\n {} ls \
                 s3://bucket-name/ --max-files 100",
                PROGRAM_NAME
            ))
            .arg(
                Arg::new("region")
                    .long("region")
                    .short('r')
                    .help("Region to use"),
            )
            .subcommand(
                Command::new("ls")
                    .about("List objects on Local Filesystem or an S3 bucket")
                    .arg(Arg::new("uri").index(1).required(true).help(
                        "URI to list objects from. E.g. s3://bucket-name/",
                    ))
                    .arg(Arg::new("name").long("name").short('n').help(
                        "Filter objects based on name. E.g. 'foo', 'foo.*', \
                         '.*bar'",
                    ))
                    .arg(
                        Arg::new("size")
                            .long("size")
                            .short('s')
                            .num_args(1)
                            .allow_hyphen_values(true)
                            .help(
                                "Filter objects based on size. E.g. '-1K', \
                                '+4M', '+1G', '-1G', '5G', '1G-2G'",
                            ),
                    )
                    .arg(
                        Arg::new("mtime")
                            .long("mtime")
                            .short('t')
                            .num_args(1)
                            .allow_hyphen_values(true)
                            .help(
                                "Filter objects based on the time offset. \
                                 E.g. '-60s', '+5m', '-1h', '+2D', '-3W', '+1M', '-1Y'",
                            ),
                    )
                    .arg(
                        Arg::new("recursive")
                            .long("recursive")
                            .short('r')
                            .action(ArgAction::SetTrue)
                            .help("List (virtual) subdirectories recursively"),
                    )
                    .arg(
                        Arg::new("max_files")
                            .long("max-files")
                            .short('m')
                            .default_value("1000")
                            .help("Maximum number of files to list"),
                    ),
            );
    let matches = app.try_get_matches_from(args).unwrap_or_else(|e| {
        e.exit();
    });

    let region = matches.get_one::<String>("region").map(ToString::to_string);

    if let Some(ls_matches) = matches.subcommand_matches("ls") {
        handle_ls(ls_matches, region);
    }
}
