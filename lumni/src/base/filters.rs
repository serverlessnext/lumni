use regex::Regex;

use crate::utils::time::system_time_in_seconds;
use crate::utils::time_parse::calculate_time_offset_seconds;
use crate::FileObject;

const BYTE_UNITS: &[(&str, u64)] = &[
    ("b", 1u64),
    ("k", 1024u64),
    ("M", 1024u64 * 1024u64),
    ("G", 1024u64 * 1024u64 * 1024u64),
    ("T", 1024u64 * 1024u64 * 1024u64 * 1024u64),
];
const PERCENTAGE: f64 = 0.05;

#[derive(Debug, Clone)]
pub struct Conditions {
    pub name_regex: Option<Regex>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_mtime: Option<u64>,
    pub max_mtime: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct FileObjectFilter {
    pub conditions: Vec<Conditions>,
}

impl FileObjectFilter {
    pub fn new(conditions: Conditions) -> Self {
        FileObjectFilter {
            conditions: vec![conditions],
        }
    }

    pub fn new_with_single_condition(
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
            Some(m) => parse_time(m, system_time_in_seconds())?,
            None => (None, None),
        };

        Ok(FileObjectFilter {
            conditions: vec![Conditions {
                name_regex,
                min_size,
                max_size,
                min_mtime,
                max_mtime,
            }],
        })
    }
    pub fn add_or_condition(&mut self, condition: Conditions) {
        self.conditions.push(condition);
    }

    pub fn matches(&self, file_object: &FileObject) -> bool {
        self.conditions.iter().any(|condition| {
            let name_match = condition
                .name_regex
                .as_ref()
                .map_or(true, |re| re.is_match(file_object.name()));

            let size_match = (condition
                .min_size
                .map_or(true, |min| file_object.size() >= min))
                && (condition
                    .max_size
                    .map_or(true, |max| file_object.size() <= max));

            let mtime_match =
                (condition.min_mtime.map_or(true, |min| {
                    file_object.modified().map_or(false, |mtime| mtime >= min)
                })) && (condition.max_mtime.map_or(true, |max| {
                    file_object.modified().map_or(false, |mtime| mtime <= max)
                }));

            name_match && size_match && mtime_match
        })
    }
}

pub fn parse_size(size: &str) -> Result<(Option<u64>, Option<u64>), String> {
    if size.contains("..") {
        // Handle range case
        let parts: Vec<&str> = size.split("..").collect();
        if parts.len() != 2 {
            return Err(format!("Invalid size range format: {}", size));
        }
        let start = if parts[0].is_empty() {
            None
        } else {
            Some(parse_single_size(parts[0])?)
        };
        let end = if parts[1].is_empty() {
            None
        } else {
            Some(parse_single_size(parts[1])?)
        };
        return Ok((start, end));
    }

    let re = Regex::new(
        r"^(?P<modifier>[+<>=-]?=?)(?P<value>\d+)(?P<unit>[bBkKmMGtT]?)$",
    )
    .unwrap();

    if let Some(caps) = re.captures(size) {
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
            "+" | ">=" => Ok((Some(size), None)),
            "-" | "<=" => Ok((None, Some(size))),
            "<" => Ok((None, Some(size.saturating_sub(1)))),
            ">" => Ok((Some(size + 1), None)),
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

fn parse_single_size(size: &str) -> Result<u64, String> {
    let re = Regex::new(r"^(?P<value>\d+)(?P<unit>[bBkKmMGtT]?)$").unwrap();
    if let Some(caps) = re.captures(size) {
        let value: u64 = caps["value"].parse().expect("Invalid numeric value");
        let unit = caps["unit"].to_ascii_lowercase();
        let multiplier = BYTE_UNITS
            .iter()
            .find(|(u, _)| u.to_lowercase() == unit)
            .map(|(_, m)| m)
            .unwrap_or(&1u64);
        Ok(value * multiplier)
    } else {
        Err(format!("Invalid size format: {}", size))
    }
}

pub fn parse_time(
    time_offset_str: &str,
    current_time: u64,
) -> Result<(Option<u64>, Option<u64>), String> {
    let is_negative = time_offset_str.starts_with('-');
    let is_positive = time_offset_str.starts_with('+');
    if time_offset_str
        .chars()
        .any(|c| !c.is_ascii_digit() && !"+-YMWDhms".contains(c))
    {
        return Err(format!("Invalid time offset string: {}", time_offset_str));
    }

    let time_offset_str = if is_negative || is_positive {
        &time_offset_str[1..]
    } else {
        time_offset_str
    };

    let total_offset_seconds =
        calculate_time_offset_seconds(time_offset_str)? as i64;

    let total_offset_seconds = if is_negative {
        -total_offset_seconds
    } else {
        total_offset_seconds
    };

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

    fn generate_valid_cases<'a>(
        current_time: u64,
        inputs: &'a [&'a str],
    ) -> Result<Vec<(&'a str, Option<u64>, Option<u64>)>, String> {
        inputs
            .iter()
            .map(|&input| {
                let is_negative = input.starts_with('-');
                let time_offset_str =
                    if is_negative { &input[1..] } else { input };
                let offset = calculate_time_offset_seconds(time_offset_str)?;
                let min_time = if is_negative {
                    Some(current_time - offset)
                } else {
                    None
                };
                let max_time = if !is_negative {
                    Some(current_time - offset)
                } else {
                    None
                };
                Ok((input, min_time, max_time))
            })
            .collect()
    }

    #[test]
    fn test_parse_time() {
        let current_time = system_time_in_seconds();

        let valid_base_cases_inputs = &[
            "2M", "-2M", "3W", "-3W", "5D", "-5D", "48h", "-48h", "20m",
            "-20m", "10s", "-10s", "1Y", "-1Y",
        ];
        let valid_base_cases =
            generate_valid_cases(current_time, valid_base_cases_inputs)
                .unwrap();

        for (input, min_time, max_time) in valid_base_cases {
            let (min_time_result, max_time_result) =
                parse_time(input, current_time).unwrap();
            assert_eq!(min_time_result, min_time);
            assert_eq!(max_time_result, max_time);
        }

        let valid_combined_cases_inputs = &[
            "1Y2M",
            "-1Y2M",
            "1D8h20m",
            "-1D8h20m",
            "3W5D",
            "-3W5D",
            "2M10D2h",
            "-2M10D2h",
            "1Y1M1W1D1h1m1s",
            "-1Y1M1W1D1h1m1s",
        ];
        let valid_combined_cases =
            generate_valid_cases(current_time, valid_combined_cases_inputs)
                .unwrap();
        for (input, min_time, max_time) in valid_combined_cases {
            let (min_time_result, max_time_result) =
                parse_time(input, current_time).unwrap();
            assert_eq!(min_time_result, min_time);
            assert_eq!(max_time_result, max_time);
        }

        // Test invalid inputs
        let invalid_cases = vec![
            "1Y2M3",
            "2d5h6m7",
            "+3D4H",
            "2.5D",
            "1M-1D",
            "10050Y",
            "-10050Y",
            "3660001D",
            "-3660001D",
            "316224000001s",
            "-316224000001s", // edge cases
            " 2M",
            "2M ",
            " 2M ",
            "\t2M",
            "2M\t", // whitespace cases
            "2H",
            "3w",
            "3y", // incorrect capitalization cases
        ];
        for input in invalid_cases {
            assert!(parse_time(input, current_time).is_err());
        }
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
        let invalid_cases =
            vec!["1G2M3k", "2g5m6b7", "1M-1K", "+3G4M", "2.5G", "5P", "5G5M"];
        for input in invalid_cases {
            assert!(parse_size(input).is_err());
            println!("input: {}", input);
        }
    }
}
