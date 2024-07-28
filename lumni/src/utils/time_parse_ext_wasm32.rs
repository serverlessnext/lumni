use js_sys::Date;
use wasm_bindgen::prelude::*;

pub fn rfc3339_to_epoch(timestamp: &str) -> Result<i64, JsValue> {
    let date = Date::new(&JsValue::from_str(timestamp));
    Ok((date.get_time() / 1000.0) as i64)
}

pub fn epoch_to_rfc3339(timestamp: i64) -> Result<String, JsValue> {
    let date = Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
    let date_string = date.to_iso_string().as_string().unwrap();
    Ok(date_string[0..19].replace("T", " ") + "Z")
}

pub fn datetime_utc() -> (u32, u8, u8, u8, u8, u8) {
    let timestamp = Date::now();
    let date = Date::new(&JsValue::from_f64(timestamp));
    (
        date.get_utc_full_year() as u32,
        (date.get_utc_month() + 1) as u8,
        date.get_utc_date() as u8,
        date.get_utc_hours() as u8,
        date.get_utc_minutes() as u8,
        date.get_utc_seconds() as u8,
    )
}
