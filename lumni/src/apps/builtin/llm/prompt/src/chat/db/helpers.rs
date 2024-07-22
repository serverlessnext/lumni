use std::time::{SystemTime, UNIX_EPOCH};

pub fn system_time_in_milliseconds() -> i64 {
    // convert to i64 to match the type of the timestamp supported by the database.
    // i64 is still large enough to count millions of years in milliseconds.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as i64
}
