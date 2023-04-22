use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn rfc3339_to_epoch(timestamp: &str) -> Result<u64, time::Error> {
    let datetime = OffsetDateTime::parse(timestamp, &Rfc3339)?;
    Ok(datetime.unix_timestamp() as u64)
}

pub fn epoch_to_rfc3339(timestamp: u64) -> Result<String, time::Error> {
    let datetime = OffsetDateTime::from_unix_timestamp(timestamp as i64)?;
    Ok(datetime.to_string())
}

pub fn datetime_utc() -> (u32, u8, u8, u8, u8, u8) {
    let time = OffsetDateTime::now_utc();
    (
        time.year() as u32,
        time.month() as u8,
        time.day(),
        time.hour(),
        time.minute(),
        time.second(),
    )
}


