use std::fs;

use lumni::api::env::ApplicationEnv;
use lumni::api::spec::ApplicationSpec;
use lumni::api::{find_builtin_app, get_app_handler, get_available_apps};
use lumni::EnvironmentConfig;
use regex::Regex;

pub async fn handle_apps(
    _matches: &clap::ArgMatches,
    _config: &mut EnvironmentConfig,
) {
    let apps = get_available_apps();
    println!("Available apps:");
    for app in apps {
        println!("{:?}", app);
    }
}

pub async fn handle_application(
    app: &str, // can be either app_name or app_uri
    mut app_env: ApplicationEnv,
    matches: &clap::ArgMatches,
) {
    let uri_pattern = Regex::new(r"^[-a-z]+::[-a-z0-9]+::[-a-z0-9]+$").unwrap();

    let app_handler = if uri_pattern.is_match(app) {
        get_app_handler(app) // app is an URI
    } else {
        find_builtin_app(app) // app is a name
    };

    match app_handler {
        Some(app_handler) => {
            // convert ArgMatches to Vec<String>, this allows Apps to choose/ implement
            // their own argument parser
            let mut app_arguments = Vec::new();
            app_arguments.push(app.to_string());

            let extra_arguments = matches
                .get_raw("")
                .unwrap_or_default()
                .map(|os_str| os_str.to_str().unwrap_or("[Invalid UTF-8]"))
                .map(String::from)
                .collect::<Vec<String>>();
            app_arguments.extend(extra_arguments);
            let app_spec = match serde_yaml::from_str::<ApplicationSpec>(
                app_handler.load_specification(),
            ) {
                Ok(spec) => spec,
                Err(_) => {
                    // this should not happen as the spec is validated at compile time
                    panic!("Failed to load specification.");
                }
            };

            if let Some(config_dir) = app_env.get_config_dir() {
                // create an apps/<app_name> subdirectory for the app's configuration
                let mut app_config_dir = config_dir.to_path_buf();
                app_config_dir.push("apps");
                app_config_dir.push(app_spec.name().to_lowercase());
                // ensure the directory exists
                fs::create_dir_all(&app_config_dir)
                    .expect("Failed to create app config directory");
                app_env.set_config_dir(app_config_dir);
            }

            let app_run = app_handler
                .invoke_main(app_spec, app_env, app_arguments)
                .await;
            match app_run {
                Ok(_) => {} // app ran successfully
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("app not found: {}", app);
            std::process::exit(1);
        }
    }
}
