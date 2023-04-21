// TEMPORARILY - this likely needs to be replaced in Web builds

use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;

use crate::FileObject;

#[derive(Debug, Clone)]
pub struct FileObjectFilter {
    name_regex: Option<Regex>,
    min_size: Option<u64>,
    max_size: Option<u64>,
    min_mtime: Option<u64>,
    max_mtime: Option<u64>,
}

fn sytem_time_in_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

impl FileObjectFilter {
    pub fn new(
        name: Option<&str>,
        size: Option<&str>,
        mtime: Option<&str>,
    ) -> Result<Self, String> {
        let name_regex = name.map(|pattern| Regex::new(pattern).unwrap());

        let (min_size, max_size) = match size {
            Some(s) => parse_size(s)?,
            None => (None, None),
        };

        let (min_mtime, max_mtime) = match mtime {
            Some(m) => parse_time(m, sytem_time_in_seconds())?,
            None => (None, None),
        };

        Ok(FileObjectFilter {
            name_regex,
            min_size,
            max_size,
            min_mtime,
            max_mtime,
        })
    }

    pub fn matches(&self, file_object: &FileObject) -> bool {
        let name_match = match &self.name_regex {
            Some(re) => re.is_match(file_object.name()),
            None => true,
        };

        let size_match = {
            (self.min_size.map_or(true, |min| file_object.size() >= min))
                && (self.max_size.map_or(true, |max| file_object.size() <= max))
        };

        let mtime_match = {
            (self.min_mtime.map_or(true, |min| {
                file_object.modified().map_or(false, |mtime| mtime >= min)
            })) && (self.max_mtime.map_or(true, |max| {
                file_object.modified().map_or(false, |mtime| mtime <= max)
            }))
        };

        name_match && size_match && mtime_match
    }
}

fn parse_size(size: &str) -> Result<(Option<u64>, Option<u64>), String> {
    const BYTE_UNITS: &[(&str, u64)] = &[
        ("b", 1u64),
        ("k", 1024u64),
        ("M", 1024u64 * 1024u64),
        ("G", 1024u64 * 1024u64 * 1024u64),
        ("T", 1024u64 * 1024u64 * 1024u64 * 1024u64),
    ];
    const PERCENTAGE: f64 = 0.05;

    let re = Regex::new(
        r"^(?P<modifier>[+\-=]?)(?P<value>\d+)(?P<unit>[bBkKmMGtT]?)$",
    )
    .unwrap();
    let range_re = Regex::new(r"^(?P<min_value>\d+)(?P<unit1>[bBkKmMGtT]?)-(?P<max_value>\d+)(?P<unit2>[bBkKmMGtT]?)$").unwrap();

    if let Some(caps) = range_re.captures(size) {
        // Handle range case
        let min_value: u64 =
            caps["min_value"].parse().expect("Invalid numeric value");
        let max_value: u64 =
            caps["max_value"].parse().expect("Invalid numeric value");
        let unit1 = caps["unit1"].to_ascii_lowercase();
        let unit2 = caps["unit2"].to_ascii_lowercase();

        let multiplier1 = BYTE_UNITS
            .iter()
            .find(|(u, _)| u.to_lowercase() == unit1)
            .map(|(_, m)| m)
            .unwrap_or(&1u64);

        let multiplier2 = BYTE_UNITS
            .iter()
            .find(|(u, _)| u.to_lowercase() == unit2)
            .map(|(_, m)| m)
            .unwrap_or(&1u64);

        let min_size = min_value * multiplier1;
        let max_size = max_value * multiplier2;

        if min_size > max_size {
            Err(format!("Minimum size is greater than maximum size."))
        } else {
            Ok((Some(min_size), Some(max_size)))
        }
    } else if let Some(caps) = re.captures(size) {
        let modifier = &caps["modifier"];
        let value: u64 = caps["value"].parse().expect("Invalid numeric value");
        let unit = caps["unit"].to_ascii_lowercase();

        let multiplier = BYTE_UNITS
            .iter()
            .find(|(u, _)| u.to_lowercase() == unit)
            .map(|(_, m)| m)
            .unwrap_or(&1u64);

        let size = value * multiplier;

        match modifier {
            "+" => Ok((Some(size), None)),
            "-" => Ok((None, Some(size))),
            "=" => Ok((Some(size), Some(size))),
            "" => {
                let min_size = (size as f64 * (1.0 - PERCENTAGE)).ceil() as u64;
                let max_size =
                    (size as f64 * (1.0 + PERCENTAGE)).floor() as u64;
                Ok((Some(min_size), Some(max_size)))
            }
            _ => Err(format!("Invalid modifier: {}", modifier)),
        }
    } else {
        Err(format!("Invalid size string: {}", size))
    }
}

fn parse_time(
    time_offset_str: &str,
    current_time: u64,
) -> Result<(Option<u64>, Option<u64>), String> {
    const TIME_UNITS: &[(&str, f64)] = &[
        ("Y", 365.25 * 86400.0),
        ("M", 30.5 * 86400.0),
        ("W", 7.0 * 86400.0),
        ("D", 86400.0),
        ("h", 3600.0),
        ("m", 60.0),
        ("s", 1.0),
    ];
    let re = Regex::new(r"(?P<value>\d+)(?P<unit>[YMWDhms])").unwrap();
    let mut total_offset_seconds = 0i64;
    for caps in re.captures_iter(time_offset_str) {
        let value: u64 = caps["value"].parse().expect("Invalid numeric value");
        let unit = &caps["unit"];
        let seconds_multiplier =
            TIME_UNITS.iter().find(|(u, _)| u == &unit).unwrap().1;
        total_offset_seconds +=
            (value as f64 * seconds_multiplier).round() as i64;
    }

    if re.find_iter(time_offset_str).count() == 0 {
        return Err(format!("Invalid time offset string: {}", time_offset_str));
    }

    if time_offset_str.starts_with('-') {
        total_offset_seconds = -total_offset_seconds;
    }


    let min_time = if total_offset_seconds < 0 {
        current_time.checked_sub(total_offset_seconds.unsigned_abs())
    } else {
        None
    };
    let max_time = if total_offset_seconds > 0 {
        current_time.checked_sub(total_offset_seconds.unsigned_abs())
    } else {
        None
    };
    Ok((min_time, max_time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {

        let current_time = sytem_time_in_seconds();

        let valid_cases: Vec<(&str, Option<u64>, Option<u64>)> = vec![
            ("1Y", None, Some(current_time - 31557600)),
            ("-1Y", Some(current_time - 31557600), None),
            ("1Y2M", None, Some(current_time - 36828000)),
            ("-1Y2M", Some(current_time - 36828000), None),
        ];


        for (input, min_time, max_time) in valid_cases {
            let (min_time_result, max_time_result) = parse_time(input, current_time).unwrap();
            assert_eq!(min_time_result, min_time);
            assert_eq!(max_time_result, max_time);
        }
        // TODO - fix more test cases
        // Test valid inputs
//        let cases = vec![
//            ("1Y", Some(31536000), None),
//            ("-1M", None, Some(2629746)),
//            ("1W", None, Some(604800)),
//            ("-2D", Some(172800), None),
//            ("1D8h20m", None, Some(110400)),
//            ("-3W5D", Some(2252800), None),
//            ("1h", None, Some(3600)),
//            ("2m", None, Some(120)),
//            ("-3s", Some(3), None),
//        ];


        // Test invalid inputs
//        let invalid_cases = vec!["1Y2M3", "2d5h6m7", "1M-1D", "+3D4H", "2.5D"];
//        for input in invalid_cases {
//            assert!(parse_time(input, current_time).is_err());
//        }
    }

    #[test]
    fn test_parse_size() {
        // Test valid inputs
        let cases = vec![
            ("+1G", Some(1073741824), None),
            ("-1G", None, Some(1073741824)),
            ("=1G", Some(1073741824), Some(1073741824)),
            ("1G-2G", Some(1073741824), Some(2147483648)),
            ("500M", Some(498073600), Some(550502400)),
            ("-5k", None, Some(5120)),
            ("100", Some(95), Some(105)),
            ("100B", Some(95), Some(105)),
        ];

        for (input, min_size, max_size) in cases {
            let (actual_min, actual_max) = parse_size(input).unwrap();
            assert_eq!(actual_min, min_size);
            assert_eq!(actual_max, max_size);
        }

        // Test invalid inputs
        let invalid_cases = vec![
            "1G2M3k", "2g5m6b7", "1M-1K", "+3G4M", "2.5G", "5P", "5G5M",
        ];
        for input in invalid_cases {
            assert!(parse_size(input).is_err());
            println!("input: {}", input);
        }
    }
}


