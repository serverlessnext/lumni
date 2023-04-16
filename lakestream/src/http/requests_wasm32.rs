use std::collections::HashMap;
use std::error::Error;

// stub function for wasm32
pub fn http_get_request(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<(String, u16), Box<dyn Error>> {
    Ok((String::new(), 0))
}

// stub function for wasm32
pub fn http_get_request_with_headers(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<(String, u16, HashMap<String, String>), Box<dyn Error>> {
    Ok((String::new(), 0, HashMap::new()))
}
