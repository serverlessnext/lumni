use lakestream::EnvironmentConfig;

pub async fn handle_cp(matches: &clap::ArgMatches, _config: &mut EnvironmentConfig) {
    let source = matches.get_one::<String>("source").unwrap();
    let target = matches.get_one::<String>("target").unwrap();
    println!("Copying from {} to {}", source, target);
}
