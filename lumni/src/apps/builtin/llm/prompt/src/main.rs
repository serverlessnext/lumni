use lumni::api::spec::ApplicationSpec;
use prompt::app::run_cli;

#[tokio::main]
async fn main() {
    let spec = ApplicationSpec::default();
    let args: Vec<String> = std::env::args().collect();
    run_cli(spec, args).await.unwrap();
}