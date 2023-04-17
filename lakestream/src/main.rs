use std::env;

use lakestream::cli;

fn main() {
    let args: Vec<String> = env::args().collect();
    cli::run_cli(args);
}
