use hyper::{Body, Client, Request};
use hyper::header::{HeaderName, HeaderValue};
use hyper_tls::HttpsConnector;
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use tokio::runtime::Runtime;

type HttpResult = Result<(String, u16, HashMap<String, String>), Box<dyn Error>>;
type HttpResultWithoutHeaders = Result<(String, u16), Box<dyn Error>>;

pub async fn http_get_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> HttpResultWithoutHeaders {
    let (body, status, _) = http_get_request_with_headers(url, headers).await?;
    Ok((body, status))
}


pub async fn http_get_request_with_headers(
    url: &str,
    headers: &HashMap<String, String>,
) -> HttpResult {
    // Create an HTTPS connector and client
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, Body>(https);

    // Create a request with headers
    let mut request = Request::get(url).body(Body::empty()).unwrap();
    for (key, value) in headers {
        if let (Ok(header_name), Ok(header_value)) =
            (HeaderName::from_str(key), HeaderValue::from_str(value))
        {
            request.headers_mut().append(header_name, header_value);
        }
    }

    // Send the request and get the response
    let response = client.request(request).await?;

    let status = response.status().as_u16();
    let headers_map = parse_response_headers(&response);

    if !(200..300).contains(&status) {
        return Ok((String::new(), status, headers_map));
    }

    let body_bytes = hyper::body::to_bytes(response.into_body()).await?;
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    Ok((body, status, headers_map))
}

fn parse_response_headers(response: &hyper::Response<Body>) -> HashMap<String, String> {
    let mut headers_map = HashMap::new();

    for (key, value) in response.headers() {
        headers_map.insert(
            key.to_string(),
            value.to_str().unwrap_or_default().to_string(),
        );
    }
    headers_map
}

