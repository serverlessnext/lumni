use std::collections::HashMap;
use std::env;

use lakestream::cli;

fn main() {
    let args: Vec<String> = env::args().collect();

    let access_key = env::var("AWS_ACCESS_KEY_ID")
        .expect("Missing environment variable AWS_ACCESS_KEY_ID");
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
        .expect("Missing environment variable AWS_SECRET_ACCESS_KEY");

    let mut config = HashMap::new();
    config.insert("access_key".to_string(), access_key);
    config.insert("secret_key".to_string(), secret_key);

    cli::run_cli(args);
}
