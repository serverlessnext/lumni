mod cli;
use std::env;

use cli::parser::run_cli;

fn main() {
    let args: Vec<String> = env::args().collect();
    run_cli(args);
}
