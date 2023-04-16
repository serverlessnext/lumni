// stub function for wasm32
pub fn rfc3339_to_epoch(timestamp: &str) -> Result<u64, String> {
    Ok(0)
}

// stub function for wasm32
pub fn epoch_to_rfc3339(
    timestamp: u64,
) -> Result<String, std::num::ParseIntError> {
    Ok("20230101".to_string())
}

// stub function for wasm32
pub fn datetime_utc() -> (u32, u8, u8, u8, u8, u8) {
    (2023, 1, 1, 0, 0, 0)
}
