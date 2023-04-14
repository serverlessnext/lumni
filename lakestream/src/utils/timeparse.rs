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
