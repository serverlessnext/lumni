use std::time::{SystemTime, UNIX_EPOCH};

use time::{format_description, OffsetDateTime};

use super::time_parse_ext::{epoch_to_rfc3339, rfc3339_to_epoch};
use crate::api::error::LumniError;

pub struct Timestamp {
    pub timestamp: i64, // epoch in milliseconds
}

impl Timestamp {
    pub fn new(timestamp: i64) -> Self {
        Timestamp { timestamp }
    }

    pub fn from_system_time() -> Result<Self, LumniError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LumniError::Any(format!("SystemTime error: {}", e)))
            .and_then(|duration| {
                i64::try_from(duration.as_millis())
                    .map(|timestamp| Timestamp { timestamp })
                    .map_err(|_| {
                        LumniError::Any("Timestamp overflow".to_string())
                    })
            })
    }

    pub fn format(&self, format_str: &str) -> Result<String, LumniError> {
        let datetime =
            OffsetDateTime::from_unix_timestamp(self.timestamp / 1000)
                .map_err(|e| {
                    LumniError::Any(format!("Invalid timestamp: {}", e))
                })?;

        let format = format_description::parse(format_str).map_err(|e| {
            LumniError::Any(format!("Invalid format string: {}", e))
        })?;

        datetime
            .format(&format)
            .map_err(|e| LumniError::Any(format!("Formatting error: {}", e)))
    }

    pub fn as_millis(&self) -> i64 {
        self.timestamp
    }

    pub fn as_seconds(&self) -> i64 {
        self.timestamp / 1000
    }

    pub fn rfc3339_to_epoch(timestamp: &str) -> Result<i64, LumniError> {
        rfc3339_to_epoch(timestamp).map_err(|e| LumniError::Any(e.to_string()))
    }

    pub fn epoch_to_rfc3339(timestamp: i64) -> Result<String, LumniError> {
        epoch_to_rfc3339(timestamp).map_err(|e| LumniError::Any(e.to_string()))
    }
}
