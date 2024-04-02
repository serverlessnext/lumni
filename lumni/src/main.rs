use std::env;

use lumni::run_cli;

fn main() {
    let args: Vec<String> = env::args().collect();
    run_cli(args);
}
