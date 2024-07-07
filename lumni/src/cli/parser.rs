use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};

use clap::{Arg, Command};
use lumni::api::env::ApplicationEnv;
use lumni::EnvironmentConfig;

use super::subcommands::app::*;
use super::subcommands::cp::*;
use super::subcommands::ls::*;
use super::subcommands::query::*;
use super::subcommands::request::*;

const PROGRAM_NAME: &str = "Lumni";

pub async fn run_cli(_args: Vec<String>) {
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
        .subcommand(apps_subcommand()) // "app"
        .allow_external_subcommands(true);

    let matches = app.try_get_matches();

    match matches {
        Ok(matches) => {
            let mut config = create_initial_config(&matches);
            match matches.subcommand() {
                Some(("-X", matches)) => {
                    // request
                    handle_request(matches, &mut config).await;
                }
                Some(("-Q", matches)) => {
                    // query
                    handle_query(matches, &mut config).await;
                }
                Some(("ls", matches)) => {
                    // list
                    handle_ls(matches, &mut config).await;
                }
                Some(("cp", matches)) => {
                    // copy
                    handle_cp(matches, &mut config).await;
                }
                Some(("apps", matches)) => {
                    // show list of apps
                    handle_apps(matches, &mut config).await;
                }
                Some((app_name, matches)) => {
                    // catch all other subcommands as an App
                    let mut app_env = ApplicationEnv::new();
                    app_env.set_config_dir(get_config_dir());
                    handle_application(app_name, app_env, matches).await;
                }
                None => {
                    // given the `arg_required_else_help(true)` is defined,
                    // this branch should never be reached
                    unreachable!("arg_required_else_help(true) not defined")
                }
            }
        }
        Err(e) => {
            if e.kind() == clap::error::ErrorKind::DisplayHelp
                || e.kind() == clap::error::ErrorKind::DisplayVersion
            {
                // catches --help and --version, which are not errors
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

fn get_config_dir() -> PathBuf {
    // Check for user specified config directory in environment
    if let Ok(custom_dir) =
        env::var(format!("{}_CONFIG_DIR", PROGRAM_NAME.to_uppercase()))
    {
        return PathBuf::from(custom_dir);
    }
    if let Ok(custom_dir) =
        env::var(format!("{}_CONFIG_DIR", PROGRAM_NAME.to_uppercase()))
    {
        let path = PathBuf::from(custom_dir);
        fs::create_dir_all(&path)
            .expect("Failed to create custom config directory");
        return path;
    }

    // if no user specified config directory is found, use the default
    let home = env::var("HOME").expect("HOME environment variable not set");
    let base_path = PathBuf::from(home);

    let mut config_path = match env::consts::OS {
        "macos" => base_path.join("Library/Application Support"),
        _ => base_path.join(".config"), // Linux, other Unixes
    };
    config_path.push(PROGRAM_NAME.to_lowercase());
    fs::create_dir_all(&config_path)
        .expect("Failed to create config directory");
    config_path
}
