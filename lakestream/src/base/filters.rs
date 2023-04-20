// TEMPORARILY - this likely needs to be replaced in Web builds
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use regex::Regex;

use crate::FileObject;

pub struct FileObjectFilter {
    name_pattern: Option<Regex>,
    min_size: Option<u64>,
    equal_size: Option<u64>,
    max_size: Option<u64>,
    min_modified_time: Option<u64>,
}

impl FileObjectFilter {
    pub fn new(
        name_pattern: Option<String>,
        min_size: Option<String>,
        equal_size: Option<String>,
        max_size: Option<String>,
        modified_time_offset: Option<String>,
    ) -> Self {
        let name_regex =
            name_pattern.map(|pattern| Regex::new(&pattern).unwrap());

        let min_size = min_size.map(parse_size);
        let equal_size = equal_size.map(parse_size);
        let max_size = max_size.map(parse_size);
        let min_modified_time = modified_time_offset.map(parse_time_offset);

        FileObjectFilter {
            name_pattern: name_regex,
            min_size,
            equal_size,
            max_size,
            min_modified_time,
        }
    }

    pub fn matches(&self, file_object: &FileObject) -> bool {
        let name_match = match &self.name_pattern {
            Some(re) => re.is_match(&file_object.name()),
            None => true,
        };

        let size_match = {
            (self.min_size.map_or(true, |min| file_object.size() >= min))
                && (self.max_size.map_or(true, |max| file_object.size() <= max))
                && (self.equal_size.map_or(true, |eq| file_object.size() == eq))
        };

        let modified_time_match = match self.min_modified_time {
            Some(min_time) => file_object
                .modified()
                .map_or(false, |mtime| mtime > min_time),
            None => true,
        };

        name_match && size_match && modified_time_match
    }
}

fn parse_size(size_str: String) -> u64 {
    const BYTE_UNITS: &[(&str, u64)] = &[
        ("b", 1),
        ("k", 1024),
        ("M", 1024 * 1024),
        ("G", 1024 * 1024 * 1024),
        ("T", 1024 * 1024 * 1024 * 1024),
    ];

    let re = Regex::new(r"(?P<value>\d+)(?P<unit>[bBkKmMGtT]?)").unwrap();
    let caps = re.captures(&size_str).expect("Invalid size string");

    let value: u64 = caps["value"].parse().expect("Invalid numeric value");
    let unit = caps["unit"].to_ascii_lowercase();

    if unit.is_empty() {
        value
    } else {
        value
            * BYTE_UNITS
                .iter()
                .find(|(u, _)| u.to_lowercase() == unit)
                .unwrap()
                .1
    }
}

fn parse_time_offset(time_offset_str: String) -> u64 {
    const TIME_UNITS: &[(&str, u64)] = &[
        ("S", 1),
        ("M", 60),
        ("H", 3600),
        ("d", 86400),
        ("w", 604800),
    ];

    let re = Regex::new(r"(?P<value>\d+)(?P<unit>[sSmMhHdDwW])").unwrap();
    let caps = re
        .captures(&time_offset_str)
        .expect("Invalid time offset string");

    let value: u64 = caps["value"].parse().expect("Invalid numeric value");
    let unit = caps["unit"].to_ascii_uppercase();

    let seconds_multiplier = TIME_UNITS
        .iter()
        .find(|(u, _)| u.to_uppercase() == unit)
        .unwrap()
        .1;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let offset_seconds = value * seconds_multiplier;

    if current_time > offset_seconds {
        current_time - offset_seconds
    } else {
        0
    }
}
