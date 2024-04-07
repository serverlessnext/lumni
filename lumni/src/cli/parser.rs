use std::collections::HashMap;
use std::env;

use clap::{Arg, Command};
use lumni::EnvironmentConfig;
use tokio::runtime::Builder;

use super::subcommands::app::*;
use super::subcommands::cp::*;
use super::subcommands::ls::*;
use super::subcommands::query::*;
use super::subcommands::request::*;

const PROGRAM_NAME: &str = "Lumni";

pub fn run_cli(args: Vec<String>) {
    env_logger::init();
    let mut app = Command::new(PROGRAM_NAME)
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
        .subcommand(apps_subcommand()) // "app"
        .allow_external_subcommands(true);

    let matches = app.try_get_matches();

    match matches {
        Ok(matches) => {
            let mut config = create_initial_config(&matches);
            let rt =
                Builder::new_current_thread().enable_all().build().unwrap();

            match matches.subcommand() {
                Some(("-X", matches)) => {
                    // request
                    rt.block_on(handle_request(matches, &mut config));
                }
                Some(("-Q", matches)) => {
                    // query
                    rt.block_on(handle_query(matches, &mut config));
                }
                Some(("ls", matches)) => {
                    // list
                    rt.block_on(handle_ls(matches, &mut config));
                }
                Some(("cp", matches)) => {
                    // copy
                    rt.block_on(handle_cp(matches, &mut config));
                }
                Some(("apps", matches)) => {
                    // show list of apps
                    rt.block_on(handle_apps(matches, &mut config));
                }
                Some((app_name, matches)) => {
                    // catch all other subcommands as an App
                    rt.block_on(handle_application(
                        app_name,
                        matches,
                        &mut config,
                    ));
                }
                None => {
                    // given the `arg_required_else_help(true)` is defined,
                    // this branch should never be reached
                    unreachable!("arg_required_else_help(true) not defined")
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
