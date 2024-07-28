use regex::Regex;

use crate::utils::time_parse::calculate_time_offset_seconds;

const BYTE_UNITS: &[(&str, u64)] = &[
    ("b", 1u64),
    ("k", 1024u64),
    ("M", 1024u64 * 1024u64),
    ("G", 1024u64 * 1024u64 * 1024u64),
    ("T", 1024u64 * 1024u64 * 1024u64 * 1024u64),
];
const PERCENTAGE: f64 = 0.05;

pub struct ParseFilterCondition {}

impl ParseFilterCondition {
    pub fn size(size: &str) -> Result<(Option<u64>, Option<u64>), String> {
        if size.contains("..") {
            // Handle range case
            let parts: Vec<&str> = size.split("..").collect();
            if parts.len() != 2 {
                return Err(format!("Invalid size range format: {}", size));
            }
            let start = if parts[0].is_empty() {
                None
            } else {
                Some(ParseFilterCondition::single_size(parts[0])?)
            };
            let end = if parts[1].is_empty() {
                None
            } else {
                Some(ParseFilterCondition::single_size(parts[1])?)
            };
            return Ok((start, end));
        }

        let re = Regex::new(
            r"^(?P<modifier>[+<>=-]?=?)(?P<value>\d+)(?P<unit>[bBkKmMGtT]?)$",
        )
        .unwrap();

        if let Some(caps) = re.captures(size) {
            let modifier = &caps["modifier"];
            let value: u64 =
                caps["value"].parse().expect("Invalid numeric value");
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
                    let min_size =
                        (size as f64 * (1.0 - PERCENTAGE)).ceil() as u64;
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

    fn single_size(size: &str) -> Result<u64, String> {
        let re = Regex::new(r"^(?P<value>\d+)(?P<unit>[bBkKmMGtT]?)$").unwrap();
        if let Some(caps) = re.captures(size) {
            let value: u64 =
                caps["value"].parse().expect("Invalid numeric value");
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

    pub fn time(
        time_offset_str: &str,
        current_time: i64,
    ) -> Result<(Option<i64>, Option<i64>), String> {
        if time_offset_str
            .chars()
            .any(|c| !c.is_ascii_digit() && !"+-YMWDhms".contains(c))
        {
            return Err(format!(
                "Invalid time offset string: {}",
                time_offset_str
            ));
        }

        let total_offset_seconds =
            calculate_time_offset_seconds(time_offset_str)?;

        let min_time = current_time.checked_sub(total_offset_seconds);
        let max_time = current_time.checked_add(total_offset_seconds);

        Ok((min_time, max_time))
    }

    pub fn absolute_time(
        time_str: &str,
    ) -> Result<(Option<i64>, Option<i64>), String> {
        if time_str.contains("..") {
            // Handle range
            let parts: Vec<&str> = time_str.split("..").collect();
            if parts.len() != 2 {
                return Err(format!("Invalid time range format: {}", time_str));
            }
            let start =
                if parts[0].is_empty() {
                    None
                } else {
                    Some(parts[0].parse::<i64>().map_err(|_| {
                        format!("Invalid start time: {}", parts[0])
                    })?)
                };
            let end =
                if parts[1].is_empty() {
                    None
                } else {
                    Some(parts[1].parse::<i64>().map_err(|_| {
                        format!("Invalid end time: {}", parts[1])
                    })?)
                };
            return Ok((start, end));
        }

        match time_str.chars().next() {
            Some('>') => {
                if time_str.chars().nth(1) == Some('=') {
                    let timestamp =
                        time_str[2..].parse::<i64>().map_err(|_| {
                            format!("Invalid time format: {}", time_str)
                        })?;
                    Ok((Some(timestamp), None))
                } else {
                    let timestamp =
                        time_str[1..].parse::<i64>().map_err(|_| {
                            format!("Invalid time format: {}", time_str)
                        })?;
                    Ok((Some(timestamp + 1), None)) // Exclusive lower bound
                }
            }
            Some('<') => {
                if time_str.chars().nth(1) == Some('=') {
                    let timestamp =
                        time_str[2..].parse::<i64>().map_err(|_| {
                            format!("Invalid time format: {}", time_str)
                        })?;
                    Ok((None, Some(timestamp))) // Inclusive upper bound
                } else {
                    let timestamp =
                        time_str[1..].parse::<i64>().map_err(|_| {
                            format!("Invalid time format: {}", time_str)
                        })?;
                    Ok((None, Some(timestamp.saturating_sub(1)))) // Exclusive upper bound, converted to inclusive
                }
            }
            Some('=') => {
                let timestamp = time_str[1..].parse::<i64>().map_err(|_| {
                    format!("Invalid time format: {}", time_str)
                })?;
                Ok((Some(timestamp), Some(timestamp))) // Inclusive single point
            }
            _ => {
                // Try parsing as a direct timestamp if no prefix is found
                let timestamp = time_str.parse::<i64>().map_err(|_| {
                    format!("Invalid time format: {}", time_str)
                })?;
                Ok((Some(timestamp), Some(timestamp))) // Inclusive single point
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::time::system_time_in_seconds;

    fn generate_valid_cases<'a>(
        current_time: i64,
        inputs: &'a [&'a str],
    ) -> Result<Vec<(&'a str, Option<i64>, Option<i64>)>, String> {
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
        let current_time = system_time_in_seconds() as i64;

        let valid_base_cases_inputs = &[
            "2M", "-2M", "3W", "-3W", "5D", "-5D", "48h", "-48h", "20m",
            "-20m", "10s", "-10s", "1Y", "-1Y",
        ];
        let valid_base_cases =
            generate_valid_cases(current_time, valid_base_cases_inputs)
                .unwrap();

        for (input, min_time, max_time) in valid_base_cases {
            let (min_time_result, max_time_result) =
                ParseFilterCondition::time(input, current_time).unwrap();
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
                ParseFilterCondition::time(input, current_time).unwrap();
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
            assert!(ParseFilterCondition::time(input, current_time).is_err());
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
            let (actual_min, actual_max) =
                ParseFilterCondition::size(input).unwrap();
            assert_eq!(actual_min, min_size);
            assert_eq!(actual_max, max_size);
        }

        // Test invalid inputs
        let invalid_cases =
            vec!["1G2M3k", "2g5m6b7", "1M-1K", "+3G4M", "2.5G", "5P", "5G5M"];
        for input in invalid_cases {
            assert!(ParseFilterCondition::size(input).is_err());
            println!("input: {}", input);
        }
    }
}
