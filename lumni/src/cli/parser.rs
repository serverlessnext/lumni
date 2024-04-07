use std::collections::HashMap;
use std::env;

use clap::{Arg, Command};
use lumni::EnvironmentConfig;
use tokio::runtime::Builder;

use super::subcommands::cp::*;
use super::subcommands::ls::*;
use super::subcommands::app::*;
use super::subcommands::query::*;
use super::subcommands::request::*;

const PROGRAM_NAME: &str = "Lumni";

pub fn run_cli(args: Vec<String>) {
    env_logger::init();
    let app = Command::new(PROGRAM_NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .arg_required_else_help(true)
        .about(format!(
            "{}: explore, process and connect data",
            PROGRAM_NAME
        ))
        .arg(
            Arg::new("region")
                .long("region")
                .short('r')
                .help("Region to use"),
        )
        .subcommand(request_subcommand()) // "-X/--request [GET,PUT]"
        .subcommand(query_subcommand()) // "-Q/--query [SELECT,DESCRIBE]"
        .subcommand(ls_subcommand()) // "ls [URI]"
        .subcommand(cp_subcommand()) // "cp" [SOURCE] [TARGET]
        .subcommand(app_subcommand()); // "app"


    let matches = app.try_get_matches();

    match matches {
        Ok(matches) => {
            let mut config = create_initial_config(&matches);
            let rt =
                Builder::new_current_thread().enable_all().build().unwrap();

            match matches.subcommand() {
                Some(("-X", matches)) => {
                    rt.block_on(handle_request(matches, &mut config));
                }
                Some(("-Q", matches)) => {
                    rt.block_on(handle_query(matches, &mut config));
                }
                Some(("ls", matches)) => {
                    rt.block_on(handle_ls(matches, &mut config));
                }
                Some(("cp", matches)) => {
                    rt.block_on(handle_cp(matches, &mut config));
                }
                Some(("app", matches)) => {
                    rt.block_on(handle_app(matches, &mut config));
                }
                _ => {
                    eprintln!("No valid subcommand provided");
                }
            }
        }
        Err(e) => {
            if e.kind() == clap::error::ErrorKind::DisplayHelp {
                print!("{}", e);
            } else {
                eprintln!("Error parsing command-line arguments: {}", e);
                eprintln!(
                    "Please ensure you have provided all required arguments \
                     correctly."
                );
                eprintln!(
                    "For more detailed help, try running '--help' or \
                     '<subcommand> --help'."
                );
                std::process::exit(1);
            }
        }
    }
}

fn create_initial_config(matches: &clap::ArgMatches) -> EnvironmentConfig {
    let mut config_hashmap = HashMap::new();
    if let Some(region) = matches.get_one::<String>("region") {
        config_hashmap.insert("region".to_string(), region.to_string());
    }

    // Create a Config instance
    EnvironmentConfig::new(config_hashmap)
}
