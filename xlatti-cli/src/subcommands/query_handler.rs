
use xlatti::EnvironmentConfig;

pub async fn handle_query(
    matches: &clap::ArgMatches,
    _config: &mut EnvironmentConfig,
) {
    let statement = matches.get_one::<String>("statement").unwrap();

    // catch SELECT and DESCRIBE statements
    match statement.to_lowercase().as_str() {
        _ if statement.to_lowercase().starts_with("select") => {
            println!("SELECT statement not yet implemented");
        }
        _ if statement.to_lowercase().starts_with("describe") => {
            println!("DESCRIBE statement not yet implemented");
        }
        _ => {
            eprintln!("Invalid statement: {}", statement);
        }
    }
}