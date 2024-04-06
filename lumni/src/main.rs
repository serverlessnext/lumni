use std::env;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
pub use cli::run_cli;

#[cfg(feature = "cli")]
fn main() {
    let args: Vec<String> = env::args().collect();
    run_cli(args);
}

#[cfg(not(feature = "cli"))]
fn main() {
    println!("CLI feature is not enabled.");
}
