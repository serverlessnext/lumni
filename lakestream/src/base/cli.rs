use std::collections::HashMap;
use std::env;

use clap::{Arg, ArgAction, Command};
use regex::Regex;

use crate::{FileObjectFilter, ListObjectsResult, ObjectStoreHandler};

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
                    .help("Region to use"),
            )
            .subcommand(
                Command::new("ls")
                    .about("List objects in an S3 bucket")
                    .arg(Arg::new("uri").index(1).required(true).help(
                        "URI to list objects from. E.g. s3://bucket-name/",
                    ))
                    .arg(
                        Arg::new("size_filter")
                            .long("size-filter")
                            .short('s')
                            .help(
                                "Filter objects based on size. E.g. '+1G', \
                                 '-1G', '5G', '1G-2G'",
                            ),
                    )
                    .arg(
                        Arg::new("recursive")
                            .long("recursive")
                            .short('r')
                            .action(ArgAction::SetTrue)
                            .help("List (virtual) subdirectories recursively"),
                    )
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

    let region = matches.get_one::<String>("region").map(ToString::to_string);

    if let Some(ls_matches) = matches.subcommand_matches("ls") {
        let recursive =
            *ls_matches.get_one::<bool>("recursive").unwrap_or(&false);
        let uri = ls_matches.get_one::<String>("uri").unwrap().to_string();

        // let name_pattern = ls_matches
        //     .get_one::<String>("name_pattern")
        //     .map(ToString::to_string);
        // let size_filter = ls_matches
        //     .get_one::<String>("size_filter")
        //     .map(ToString::to_string);
        // let modified_time_offset = ls_matches
        //     .get_one::<String>("modified_time_offset")
        //     .map(ToString::to_string);

        let max_files = ls_matches
            .get_one::<String>("max_files")
            .unwrap()
            .parse::<u32>()
            .expect("Invalid value for max_files");

        handle_ls(uri, recursive, max_files, region);
    }
}

fn handle_ls(
    uri: String,
    recursive: bool,
    max_files: u32,
    region: Option<String>,
) {
    let mut config = HashMap::new();

    if let Some(region) = region {
        config.insert("region".to_string(), region);
    }

    // TODO: implement this via CLI
    // uncomment this to test the filter
    // let name_pattern = Some(String::from(r".*"));
    // let min_size = Some("1M".to_string());
    // let equal_size = None;
    // let max_size = Some("4M".to_string());
    // let modified_time_offset = None;
    // let filter = Some(FileObjectFilter::new(name_pattern, min_size, equal_size, max_size, modified_time_offset));
    let filter = None;

    match ObjectStoreHandler::list_objects(
        uri,
        config,
        recursive,
        Some(max_files),
        &filter,
    ) {
        ListObjectsResult::FileObjects(file_objects) => {
            // Print file objects to stdout
            println!("Found {} file objects:", file_objects.len());
            for fo in file_objects {
                let full_path = true;
                println!("{}", fo.printable(full_path));
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

fn size_filter_validator(s: &str) -> Result<(), String> {
    let regex = Regex::new(r"^([+\-]?)(\d+(?:\.\d+)?)([bkMGTP])?$").unwrap();

    if regex.is_match(s) {
        Ok(())
    } else {
        Err(format!("Invalid size filter format: {}", s))
    }
}
