use base64::engine::general_purpose;
use base64::Engine as _;
use wasm_bindgen::JsValue;

pub fn generate_password() -> Result<String, JsValue> {
    generate_string(16)
}

pub fn generate_salt() -> Result<String, JsValue> {
    generate_string(16)
}

pub fn generate_string(length: usize) -> Result<String, JsValue> {
    let mut salt = vec![0u8; length];

    web_sys::window()
        .expect("no global `window` exists")
        .crypto()
        .expect("should have a `Crypto` on the `Window`")
        .get_random_values_with_u8_array(&mut salt)
        .expect("get_random_values_with_u8_array failed");

    // Convert the salt array to a base64 string
    Ok(general_purpose::STANDARD.encode(&salt))
}
