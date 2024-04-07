use lumni::EnvironmentConfig;

use lumni::api::get_available_apps;


pub async fn handle_apps(
    _matches: &clap::ArgMatches,
    _config: &mut EnvironmentConfig,
) {
    let apps = get_available_apps();
    println!("Available apps:");
    for app in apps {
        let name = app.get("name").unwrap();
        println!("- {}", name);
    }
}


pub async fn handle_application(
    app_name: &str,
    matches: &clap::ArgMatches,
    _config: &mut EnvironmentConfig,
) {
    // TODO: validate app_name and run the app
    println!("App called: {}", app_name);
    println!("Subcommand matches: {:?}", matches);
}
