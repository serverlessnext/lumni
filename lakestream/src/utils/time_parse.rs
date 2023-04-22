
use regex::Regex;

pub fn calculate_time_offset_seconds(time_offset_str: &str) -> Result<u64, String> {
    // TODO: invalid case of 1M-1D is not caught due to regex issue
    let re = Regex::new(r"(?P<value>\d+)(?P<unit>[YMWDhms])").unwrap();

    let mut total_offset_seconds = 0i64;
    let mut remaining_str = time_offset_str;
    while let Some(caps) = re.captures(remaining_str) {
        let value: u64 = caps["value"].parse().expect("Invalid numeric value");
        let unit = &caps["unit"];
        let seconds_multiplier =
            TIME_UNITS.iter().find(|(u, _)| u == &unit).ok_or_else(|| format!("Invalid time unit: {}", unit))?.1;
        total_offset_seconds +=
            (value as f64 * seconds_multiplier).round() as i64;
        remaining_str = &remaining_str[caps.get(0).unwrap().end()..];
    }

    if !remaining_str.is_empty() {
        Err(format!("Invalid time offset string: {}", time_offset_str))
    } else if total_offset_seconds == 0 {
        Err(format!("Invalid time offset string: {}", time_offset_str))
    } else {
        Ok(total_offset_seconds.unsigned_abs() as u64)
    }
}

const TIME_UNITS: &[(&str, f64)] = &[
    ("Y", 365.25 * 86400.0),
    ("M", 30.5 * 86400.0),
    ("W", 7.0 * 86400.0),
    ("D", 86400.0),
    ("h", 3600.0),
    ("m", 60.0),
    ("s", 1.0),
];

