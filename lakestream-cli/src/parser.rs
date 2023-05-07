use std::env;

use crate::subcommands::ls::*;
use crate::subcommands::x::*;
use crate::subcommands::cp::*;

use clap::{Arg, Command};
use env_logger;
use tokio::runtime::Builder;


const PROGRAM_NAME: &str = "lakestream";

pub fn run_cli(args: Vec<String>) {
    env_logger::init();
    let app = Command::new(PROGRAM_NAME)
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
        .subcommand(x_subcommand())    // "-X [GET,PUT]"
        .subcommand(ls_subcommand())    // "ls [URI]"
        .subcommand(cp_subcommand());   // "cp" [SOURCE] [TARGET]

    let matches = app.try_get_matches_from(args).unwrap_or_else(|e| {
        e.exit();
    });

    let region = matches.get_one::<String>("region").map(ToString::to_string);


    if let Some(x_matches) = matches.subcommand_matches("-X") {
        let method = x_matches.get_one::<String>("method").unwrap();
        let uri = x_matches.get_one::<String>("uri").unwrap();
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(handle_x(&method, &uri));
    } else if let Some(ls_matches) = matches.subcommand_matches("ls") {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(handle_ls(ls_matches, region));
    } else if let Some(cp_matches) = matches.subcommand_matches("cp") {
        let source = cp_matches.get_one::<String>("source").unwrap();
        let target = cp_matches.get_one::<String>("target").unwrap();
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(handle_cp(&source, &target));
    }
}
