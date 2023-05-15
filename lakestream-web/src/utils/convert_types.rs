pub fn string_to_uint8array(s: &str) -> js_sys::Uint8Array {
    let mut buffer = vec![0; s.len()];
    buffer.copy_from_slice(s.as_bytes());
    js_sys::Uint8Array::from(&buffer[..])
}

pub fn uint8array_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| *byte as char).collect()
}
