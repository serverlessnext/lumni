pub use super::timeparse::{datetime_utc, epoch_to_rfc3339, rfc3339_to_epoch};

impl UtcTimeNow {
    pub fn new() -> UtcTimeNow {
        let (year, month, day, hour, minute, second) = datetime_utc();
        UtcTimeNow {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }
}

pub struct UtcTimeNow {
    year: u32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl UtcTimeNow {
    pub fn date_stamp(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }
    pub fn x_amz_date(&self) -> String {
        format!(
            "{}T{:02}{:02}{:02}Z",
            &self.date_stamp(),
            self.hour,
            self.minute,
            self.second
        )
    }
}
