use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[cfg(feature = "http_client")]
use lumni::HttpHandler;
use lumni::{
    BinaryCallbackWrapper, EnvironmentConfig, ObjectStoreHandler, ParsedUri,
    UriScheme,
};

pub async fn handle_request(
    matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    let method = matches.get_one::<String>("method").unwrap();
    let uri = matches.get_one::<String>("uri").unwrap();

    // TODO: implement output file option vs default stdout
    // writing to an output file works internally, but need
    // -o output_file as a main cli option applicable to all commands
    let output_file = None;

    println!("Handling request: {} {}", method, uri);
    match method.as_str() {
        "GET" => {
            handle_get_request(uri, config, output_file).await;
        }
        "PUT" => {
            println!("PUT request not yet implemented");
        }
        "DELETE" => {
            println!("DELETE request not yet implemented");
        }
        "HEAD" => {
            println!("HEAD request not yet implemented");
        }
        "LIST" => {
            println!("LIST request not yet implemented");
        }
        _ => {
            eprintln!("Invalid HTTP method: {}", method);
        }
    }
}

async fn handle_get_request(
    uri: &str,
    config: &EnvironmentConfig,
    output_path: Option<&str>,
) {
    let callback = if let Some(output_path) = output_path {
        // write to file
        let file = Arc::new(Mutex::new(File::create(output_path).unwrap()));
        Some(BinaryCallbackWrapper::create_async(move |data: Vec<u8>| {
            let mut file = file.lock().unwrap();
            if let Err(e) = file.write_all(&data) {
                eprintln!("Error writing to file: {:?}", e);
            }
            async {}
        }))
    } else {
        // write to stdout
        Some(BinaryCallbackWrapper::create_async(move |data: Vec<u8>| {
            let mut stdout = io::stdout();
            if let Err(e) = stdout.write_all(&data) {
                eprintln!("Error writing to stdout: {:?}", e);
            }
            async {}
        }))
    };

    println!("GET request to: {}", uri);
    let parsed_uri = ParsedUri::from_uri(uri, false);
    println!("Parsed URI: {}", parsed_uri.to_string());

    match parsed_uri.scheme {
        UriScheme::S3 | UriScheme::LocalFs => {
            // Handler logic for both S3 and LocalFs
            let handler = ObjectStoreHandler::new(None);
            if let Err(err) =
                handler.get_object(&parsed_uri, config, callback).await
            {
                eprintln!("Error: {:?}", err);
            }
        }
        #[cfg(feature = "http_client")] // HTTP client feature enabled
        UriScheme::Http | UriScheme::Https => {
            let handler = HttpHandler::new(callback);
            if let Err(err) = handler.get(uri).await {
                eprintln!("Error: {:?}", err);
            }
        }
        #[cfg(not(feature = "http_client"))] // HTTP client feature not enabled
        UriScheme::Http | UriScheme::Https => {
            eprintln!(
                "HTTP and HTTPS support is not enabled. Please enable the \
                 `http_client` feature to use this functionality."
            );
        }
        _ => {
            // Handle unsupported schemes
            eprintln!("Unsupported scheme: {}", parsed_uri.scheme.to_string());
        }
    }
}
