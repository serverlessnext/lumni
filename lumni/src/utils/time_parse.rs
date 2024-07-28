use regex::Regex;

pub fn calculate_time_offset_seconds(
    time_offset_str: &str,
) -> Result<i64, String> {
    // add reasonable boundary to prevent overflow in unknown edge cases
    const MAX_OFFSET_SECONDS: i64 = 10000 * 366 * 86400;
    const MIN_OFFSET_SECONDS: i64 = -MAX_OFFSET_SECONDS;

    let re =
        Regex::new(r"^(?:(?P<sign>[+-])?(?P<value>\d+)(?P<unit>[YMWDhms]))+$")
            .unwrap();
    let mut total_offset_seconds = 0i64;
    let mut remaining_str = time_offset_str;

    while let Some(caps) = re.captures(remaining_str) {
        let sign =
            caps.name("sign")
                .map_or(1, |m| if m.as_str() == "-" { -1 } else { 1 });
        let value: i64 = caps["value"].parse().expect("Invalid numeric value");
        let unit = &caps["unit"];
        let seconds_multiplier = TIME_UNITS
            .iter()
            .find(|(u, _)| u == &unit)
            .ok_or_else(|| format!("Invalid time unit: {}", unit))?
            .1;

        total_offset_seconds +=
            sign * (value as f64 * seconds_multiplier).round() as i64;

        if !(MIN_OFFSET_SECONDS..=MAX_OFFSET_SECONDS)
            .contains(&total_offset_seconds)
        {
            return Err(format!(
                "Invalid time offset string: {} (offset exceeds valid range)",
                time_offset_str
            ));
        }

        remaining_str = &remaining_str[caps.get(0).unwrap().end()..];
    }

    if !remaining_str.is_empty() || total_offset_seconds == 0 {
        Err(format!("Invalid time offset string: {}", time_offset_str))
    } else {
        Ok(total_offset_seconds)
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
