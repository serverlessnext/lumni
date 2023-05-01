use reqwest::{Client, ClientBuilder, Response};
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;


type HttpResult = Result<(String, u16, HashMap<String, String>), Box<dyn Error>>;
type HttpResultWithoutHeaders = Result<(String, u16), Box<dyn Error>>;

pub async fn http_get_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> HttpResultWithoutHeaders {
    let response = perform_request(url, headers).await?;
    let status = response.status().as_u16();

    if !(200..300).contains(&status) {
        return Ok((String::new(), status));
    }
    let body = response.text().await?;
    Ok((body, status))
}

pub async fn http_get_request_with_headers(
    url: &str,
    headers: &HashMap<String, String>,
) -> HttpResult {
    let response = perform_request(url, headers).await?;
    let status = response.status().as_u16();
    let headers_map = parse_response_headers(&response);

    if !(200..300).contains(&status) {
        return Ok((String::new(), status, headers_map));
    }

    let body = response.text().await?;
    Ok((body, status, headers_map))
}

async fn perform_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<Response, Box<dyn Error>> {
    let client_builder = if url.starts_with("https://localhost") {
        ClientBuilder::new().danger_accept_invalid_certs(true)
    } else {
        ClientBuilder::new()
    };

    let client = client_builder.build()?;

    let mut request_builder = client.get(url);

    // Convert the HashMap<String, String> to HeaderMap
    let mut header_map = reqwest::header::HeaderMap::new();
    for (key, value) in headers {
        if let (Ok(header_name), Ok(header_value)) =
            (
                reqwest::header::HeaderName::from_str(key),
                reqwest::header::HeaderValue::from_str(value)
            )
        {
            header_map.insert(header_name, header_value);
        }
    }

    let request = request_builder.headers(header_map);

    let response = request.send().await?;
    Ok(response)
}


fn parse_response_headers(response: &Response) -> HashMap<String, String> {
    let mut headers_map = HashMap::new();

    for (key, value) in response.headers() {
        headers_map.insert(
            key.to_string(),
            value.to_str().unwrap_or_default().to_string(),
        );
    }
    headers_map
}

