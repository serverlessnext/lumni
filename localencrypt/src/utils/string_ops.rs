use base64::engine::general_purpose;
use base64::Engine as _;
use wasm_bindgen::JsValue;

pub fn generate_string(length: usize) -> Result<Vec<u8>, JsValue> {
    let mut salt = vec![0u8; length];

    web_sys::window()
        .expect("no global `window` exists")
        .crypto()
        .expect("should have a `Crypto` on the `Window`")
        .get_random_values_with_u8_array(&mut salt)
        .expect("get_random_values_with_u8_array failed");

    Ok(salt)
}

pub fn generate_string_base64(length: usize) -> Result<String, JsValue> {
    let bytes = generate_string(length)?;
    Ok(general_purpose::STANDARD.encode(&bytes))
}

pub fn generate_password_base64() -> Result<String, JsValue> {
    generate_string_base64(16)
}

pub fn generate_salt_base64() -> Result<String, JsValue> {
    generate_string_base64(16)
}

#[cfg(test)]
mod tests {
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    use wasm_bindgen_test::*;

    use super::*;

    #[wasm_bindgen_test]
    fn test_generate_string() {
        let length = 16;
        let result = generate_string(length).unwrap();
        assert_eq!(result.len(), length);
    }

    #[wasm_bindgen_test]
    fn test_generate_string_base64() {
        let length = 16;
        let result = generate_string_base64(length).unwrap();
        let decoded =
            general_purpose::STANDARD.decode(result.as_bytes()).unwrap();
        assert_eq!(decoded.len(), length);
    }

    #[wasm_bindgen_test]
    fn test_generate_password_base64() {
        let length = 16;
        let result = generate_password_base64().unwrap();
        let decoded =
            general_purpose::STANDARD.decode(result.as_bytes()).unwrap();
        assert_eq!(decoded.len(), length);
    }

    #[wasm_bindgen_test]
    fn test_generate_salt_base64() {
        let length = 16;
        let result = generate_salt_base64().unwrap();
        let decoded =
            general_purpose::STANDARD.decode(result.as_bytes()).unwrap();
        assert_eq!(decoded.len(), length);
    }
}
