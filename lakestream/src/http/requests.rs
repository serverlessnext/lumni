use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use bytes::Bytes;
use hyper::client::HttpConnector;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use native_tls::TlsConnector as NativeTlsConnector;
use tokio_native_tls::TlsConnector;
use url::Url;

type HttpResult =
    Result<(Bytes, u16, HashMap<String, String>), Box<dyn Error>>;
type HttpResultWithoutHeaders = Result<(Bytes, u16), Box<dyn Error>>;

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
    let url_u = Url::parse(url)?;
    let accept_invalid_certs = url_u.scheme() == "https"
        && url_u.host_str() == Some("localhost")
        && url_u.port().map_or(true, |port| port > 0);

    let mut native_tls_connector_builder = NativeTlsConnector::builder();
    native_tls_connector_builder
        .danger_accept_invalid_certs(accept_invalid_certs);
    let native_tls_connector = native_tls_connector_builder.build().unwrap();

    let tls_connector = TlsConnector::from(native_tls_connector);

    let mut http_connector = HttpConnector::new();
    http_connector.enforce_http(false);

    let https = HttpsConnector::from((http_connector, tls_connector));
    let client = Client::builder().build::<_, Body>(https);

    let mut request = Request::get(url).body(Body::empty())?;
    for (key, value) in headers.iter() {
        if let (Ok(header_name), Ok(header_value)) =
            (HeaderName::from_str(key), HeaderValue::from_str(value))
        {
            request.headers_mut().append(header_name, header_value);
        }
    }

    let response = client.request(request).await?;

    let status = response.status().as_u16();
    let headers_map = parse_response_headers(&response);

    if !(200..300).contains(&(status as isize)) {
        return Ok((Bytes::new(), status, headers_map));
    }
    let body_bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok((body_bytes, status, headers_map))
}

fn parse_response_headers(
    response: &hyper::Response<Body>,
) -> HashMap<String, String> {
    let mut headers_map = HashMap::new();

    for (key, value) in response.headers() {
        headers_map.insert(
            key.to_string(),
            value.to_str().unwrap_or_default().to_string(),
        );
    }
    headers_map
}
