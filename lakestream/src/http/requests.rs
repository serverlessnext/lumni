use std::collections::HashMap;
use std::error::Error;
use std::io::Read;

use ureq::{Agent, Response};

pub fn http_get_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<(String, u16), Box<dyn Error>> {
    let response = perform_request(url, headers)?;
    let status = response.status();

    if !(200..300).contains(&status) {
        return Ok((String::new(), status));
    }
    let mut body = String::new();
    response.into_reader().read_to_string(&mut body)?;
    Ok((body, status))
}

pub fn http_get_request_with_headers(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<(String, u16, HashMap<String, String>), Box<dyn Error>> {
    let response = perform_request(url, headers)?;
    let status = response.status();
    let headers_map = parse_response_headers(&response);

    if !(200..300).contains(&status) {
        return Ok((String::new(), status, headers_map));
    }

    let mut body = String::new();
    response.into_reader().read_to_string(&mut body)?;
    Ok((body, status, headers_map))
}

fn perform_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<Response, Box<dyn Error>> {
    let agent = Agent::new();
    let mut request_builder = agent.get(url);

    // Add headers to the request
    for (key, value) in headers {
        request_builder = request_builder.set(&key, &value);
    }

    let response = request_builder.call()?;
    Ok(response)
}

fn parse_response_headers(
    response: &ureq::Response,
) -> HashMap<String, String> {
    let header_names = response.headers_names();

    // Creating a HashMap to store the headers
    let mut headers_map: HashMap<String, String> = HashMap::new();

    // Iterating through the header names and getting their values
    for key in header_names {
        if let Some(value) = response.header(&key) {
            headers_map.insert(key, value.to_owned());
        }
    }
    headers_map
}
