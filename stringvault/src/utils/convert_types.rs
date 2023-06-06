pub fn string_to_uint8array(s: &str) -> js_sys::Uint8Array {
    let mut buffer = vec![0; s.len()];
    buffer.copy_from_slice(s.as_bytes());
    js_sys::Uint8Array::from(&buffer[..])
}

pub fn uint8array_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| *byte as char).collect()
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_string_to_uint8array() {
        let test_string = "test string";
        let expected_result: Vec<u8> = test_string.as_bytes().to_vec();
        let result = string_to_uint8array(test_string);
        let result_vec: Vec<u8> =
            result.to_vec().into_iter().map(|x| x as u8).collect();
        assert_eq!(result_vec, expected_result);
    }

    #[wasm_bindgen_test]
    fn test_uint8array_to_string() {
        let test_bytes = vec![116, 101, 115, 116]; // "test" in ASCII
        let expected_result = "test";
        let result = uint8array_to_string(&test_bytes);
        assert_eq!(result, expected_result);
    }
}
