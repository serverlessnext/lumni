use std::collections::HashMap;
use std::str::FromStr;

use bytes::{Bytes, BytesMut};
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Request, Response, Uri};
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

type HttpResult = Result<(Bytes, u16, HashMap<String, String>), anyhow::Error>;
type HttpResultWithoutHeaders = Result<(Bytes, u16), anyhow::Error>;

pub async fn http_get_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> HttpResultWithoutHeaders {
    let method = "GET";
    let (body, status, _) =
        http_request_with_headers(url, headers, method).await?;
    Ok((body, status))
}

pub async fn http_request_with_headers(
    url: &str,
    headers: &HashMap<String, String>,
    method: &str,
) -> HttpResult {
    let https = HttpsConnector::new();
    let client: Client<_, Empty<Bytes>> =
        Client::builder(TokioExecutor::new()).build::<_, Empty<Bytes>>(https);

    let uri = url.parse::<Uri>()?;
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Empty::<Bytes>::new())?;

    for (key, value) in headers.iter() {
        if let (Ok(header_name), Ok(header_value)) =
            (HeaderName::from_str(key), HeaderValue::from_str(value))
        {
            request.headers_mut().append(header_name, header_value);
        }
    }

    let mut response = client.request(request).await?;

    let status = response.status().as_u16();
    let headers_map = parse_response_headers(&response);

    if !(200..300).contains(&(status as isize)) {
        return Ok((Bytes::new(), status, headers_map));
    }

    let mut body_bytes = BytesMut::new();

    while let Some(next) = response.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            body_bytes.extend_from_slice(chunk);
        }
    }

    let status = response.status().as_u16();
    let headers_map =
        response
            .headers()
            .iter()
            .fold(HashMap::new(), |mut acc, (k, v)| {
                acc.insert(
                    k.to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                );
                acc
            });

    return Ok((body_bytes.into(), status, headers_map));
}

fn parse_response_headers(
    response: &Response<Incoming>,
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
