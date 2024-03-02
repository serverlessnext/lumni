use std::collections::HashMap;

use bytes::Bytes;
use js_sys::{ArrayBuffer, Uint8Array};
use log::info;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

use crate::LakestreamError;

pub async fn http_request_with_headers(
    url: &str,
    headers: &HashMap<String, String>,
    method: &str,
) -> Result<(Bytes, u16, HashMap<String, String>), LakestreamError> {
    info!("http_get_request_with_headers: {}", url);
    // TODO: implement response headers -- for now forward to http_get_request
    // Call the http_get_request function
    let (response_body, response_status) =
        http_request(url, headers, method).await?;

    // Add the headers to the returned result
    Ok((response_body, response_status, HashMap::new()))
}

pub async fn http_get_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<(Bytes, u16), LakestreamError> {
    let method = "GET";
    let (body, status) = http_request(url, headers, method).await?;
    Ok((body, status))
}


pub async fn http_request(
    url: &str,
    headers: &HashMap<String, String>,
    method: &str,
) -> Result<(Bytes, u16), LakestreamError> {
    info!("http_request: {}", url);
    let window = web_sys::window().ok_or(LakestreamError::String("No window available".to_string()))?;

    let mut request_init = RequestInit::new();
    request_init.method(method);
    request_init.mode(RequestMode::Cors);

    let headers_map = Headers::new().map_err(|e| LakestreamError::Js(e.into()))?;
    for (key, value) in headers {
        headers_map.set(key, value).map_err(|e| LakestreamError::Js(e.into()))?;
    }
    request_init.headers(&headers_map);

    let request = Request::new_with_str_and_init(url, &request_init).map_err(|e| LakestreamError::Js(e.into()))?;
    let response_js = JsFuture::from(window.fetch_with_request(&request)).await.map_err(|e| LakestreamError::Js(e.into()))?;
    let response: Response = response_js.dyn_into().map_err(|e| LakestreamError::Js(e.into()))?;

    let status = response.status();
    if status >= 200 && status < 300 {
        let body_js = JsFuture::from(response.array_buffer().map_err(|e| LakestreamError::Js(e.into()))?)
            .await
            .map_err(|e| LakestreamError::Js(e.into()))?;
        let body: js_sys::ArrayBuffer = body_js.dyn_into().map_err(|e| LakestreamError::Js(e.into()))?;

        let uint8_array = js_sys::Uint8Array::new(&body);
        let body_bytes = uint8_array.to_vec();
        Ok((Bytes::from(body_bytes), status))
    } else {
        let body_js = JsFuture::from(response.array_buffer().map_err(|e| LakestreamError::Js(e.into()))?)
            .await
            .map_err(|e| LakestreamError::Js(e.into()))?;
        let body: js_sys::ArrayBuffer = body_js.dyn_into().map_err(|e| LakestreamError::Js(e.into()))?;

        let uint8_array = js_sys::Uint8Array::new(&body);
        let vec = uint8_array.to_vec();
        let body = String::from_utf8_lossy(&vec);
        let error_message = format!("Error: {} - {}", status, body);
        Err(LakestreamError::String(error_message))
    }
}
