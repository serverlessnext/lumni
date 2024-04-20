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
    _matches: &clap::ArgMatches,
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
            let fake_args = Vec::new();
            let app_run = app_handler.invoke_main(fake_args).await;
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
