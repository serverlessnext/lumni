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
    matches: &clap::ArgMatches,
    _config: &mut EnvironmentConfig,
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
            let app_arguments = matches.get_raw("")
                .unwrap_or_default()
                .map(|os_str| os_str.to_str().unwrap_or("[Invalid UTF-8]"))
                .map(String::from)
                .collect::<Vec<String>>();
            let app_run = app_handler.invoke_main(app_arguments).await;
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
