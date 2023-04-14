use std::collections::HashMap;
use std::env;

use clap::Parser;

use crate::{ListObjectsResult, ObjectStoreHandler, DEFAULT_AWS_REGION};

const PROGRAM_NAME: &str = "lakestream";

#[derive(Parser)]
#[command(author, version, about = format!("List objects in an S3 bucket\n\nExample:\n {} ls s3://bucket-name/ --max-files 100", PROGRAM_NAME))]
struct Cli {
    #[clap(long, short, default_value = DEFAULT_AWS_REGION)]
    region: String,

    #[clap(subcommand)]
    cmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    Ls(Ls),
}

#[derive(Parser)]
struct Ls {
    #[clap(index = 1)]
    uri: String,

    #[clap(long, short, default_value = "1000")]
    max_files: u32,
}

pub fn run_cli(args: Vec<String>) {
    let cli = Cli::parse_from(args);

    let access_key = env::var("AWS_ACCESS_KEY_ID")
        .expect("Missing environment variable AWS_ACCESS_KEY_ID");
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
        .expect("Missing environment variable AWS_SECRET_ACCESS_KEY");

    let mut config = HashMap::new();
    config.insert("access_key".to_string(), access_key);
    config.insert("secret_key".to_string(), secret_key);
    config.insert("region".to_string(), cli.region);

    match cli.cmd {
        SubCommand::Ls(ls) => {
            handle_ls(ls, config);
        }
    }
}

fn handle_ls(ls: Ls, config: HashMap<String, String>) {
    match ObjectStoreHandler::list_objects(ls.uri, config, Some(ls.max_files)) {
        ListObjectsResult::FileObjects(file_objects) => {
            // Print file objects to stdout
            println!("Found {} file objects:", file_objects.len());
            for fo in file_objects {
                println!("{}", fo.printable());
            }
        }
        ListObjectsResult::Buckets(buckets) => {
            // Print buckets to stdout
            println!("Found {} buckets:", buckets.len());
            for bucket in buckets {
                println!("{}", bucket.name());
            }
        }
    }
}
