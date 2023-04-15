use std::collections::HashMap;
use std::env;

use clap::{Arg, Command};

use crate::{ListObjectsResult, ObjectStoreHandler, DEFAULT_AWS_REGION};

const PROGRAM_NAME: &str = "lakestream";

pub fn run_cli(args: Vec<String>) {
    let app =
        Command::new(PROGRAM_NAME)
            .version(env!("CARGO_PKG_VERSION"))
            .arg_required_else_help(true)
            .about(format!(
                "List objects in an S3 bucket\n\nExample:\n {} ls \
                 s3://bucket-name/ --max-files 100",
                PROGRAM_NAME
            ))
            .arg(
                Arg::new("region")
                    .long("region")
                    .short('r')
                    .default_value(DEFAULT_AWS_REGION)
                    .help("Region to use"),
            )
            .subcommand(
                Command::new("ls")
                    .about("List objects in an S3 bucket")
                    .arg(Arg::new("uri").index(1).required(true).help(
                        "URI to list objects from. E.g. s3://bucket-name/",
                    ))
                    .arg(
                        Arg::new("max_files")
                            .long("max-files")
                            .short('m')
                            .default_value("1000")
                            .help("Maximum number of files to list"),
                    ),
            );

    let matches = app.try_get_matches_from(args).unwrap_or_else(|e| {
        e.exit();
    });

    let region = matches.get_one::<String>("region").unwrap().to_string();
    let access_key = env::var("AWS_ACCESS_KEY_ID")
        .expect("Missing environment variable AWS_ACCESS_KEY_ID");
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
        .expect("Missing environment variable AWS_SECRET_ACCESS_KEY");

    let mut config = HashMap::new();
    config.insert("access_key".to_string(), access_key);
    config.insert("secret_key".to_string(), secret_key);
    config.insert("region".to_string(), region);

    if let Some(ls_matches) = matches.subcommand_matches("ls") {
        let uri = ls_matches.get_one::<String>("uri").unwrap().to_string();
        let max_files = ls_matches
            .get_one::<String>("max_files")
            .unwrap()
            .parse::<u32>()
            .expect("Invalid value for max_files");
        handle_ls(uri, max_files, config);
    }
}

fn handle_ls(uri: String, max_files: u32, config: HashMap<String, String>) {
    match ObjectStoreHandler::list_objects(uri, config, Some(max_files)) {
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
