use std::fs::File;
use std::io::{self,Write};
use std::sync::{Arc, Mutex};

use lakestream::{BinaryCallbackWrapper, Config, ObjectStoreHandler};

pub async fn handle_request(matches: &clap::ArgMatches, config: &mut Config) {
    let method = matches.get_one::<String>("method").unwrap();
    let uri = matches.get_one::<String>("uri").unwrap();

    // TODO: implement output file option vs default stdout
    // writing to an output file works internally, but need
    // -o output_file as a main cli option applicable to all commands
    let output_file = None;

    match method.as_str() {
        "GET" => {
            handle_get_request(uri, config, output_file).await;
        }
        "PUT" => {
            println!("PUT request");
        }
        "DELETE" => {
            println!("DELETE request");
        }
        "HEAD" => {
            println!("HEAD request");
        }
        "LIST" => {
            println!("LIST request");
        }
        _ => {
            eprintln!("Invalid HTTP method: {}", method);
        }
    }
}

async fn handle_get_request(
    uri: &str,
    config: &Config,
    output_path: Option<&str>,
) {
    let handler = ObjectStoreHandler::new(None);

    let callback = if let Some(output_path) = output_path {
        let file = Arc::new(Mutex::new(File::create(output_path).unwrap()));
        Some(BinaryCallbackWrapper::create_async(move |data: Vec<u8>| {
            let mut file = file.lock().unwrap();
            if let Err(e) = file.write_all(&data) {
                eprintln!("Error writing to file: {:?}", e);
            }
            async {}
        }))
    } else {
        Some(BinaryCallbackWrapper::create_async(move |data: Vec<u8>| {
            let mut stdout = io::stdout();
            if let Err(e) = stdout.write_all(&data) {
                eprintln!("Error writing to stdout: {:?}", e);
            }
            async {}
        }))
    };

    if let Err(err) = handler.get_object(uri, config, callback).await {
        eprintln!("Error: {:?}", err);
    }
}
