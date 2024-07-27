use std::fs;
use std::path::Path;

use regex::Regex;

use super::glob_matcher::GlobMatcher;
use super::{Conditions, ParseFilterCondition};
use crate::utils::time::system_time_in_seconds;
use crate::{FileObject, InternalError};

#[derive(Debug, Clone)]
pub struct FileObjectFilter {
    pub conditions: Vec<Conditions>,
    pub glob_matcher: Option<GlobMatcher>,
}

impl FileObjectFilter {
    pub fn new(conditions: Conditions) -> Self {
        FileObjectFilter {
            conditions: vec![conditions],
            glob_matcher: None,
        }
    }

    pub fn new_with_single_condition(
        name: Option<&str>,
        size: Option<&str>,
        mtime: Option<&str>,
        root_path: Option<&Path>,
        ignore_contents: Option<String>,
    ) -> Result<Self, InternalError> {
        let name_regex = name.map(|pattern| Regex::new(pattern).unwrap());

        let (min_size, max_size) = match size {
            Some(s) => ParseFilterCondition::size(s)?,
            None => (None, None),
        };

        let (min_mtime, max_mtime) = match mtime {
            Some(m) => ParseFilterCondition::time(m, system_time_in_seconds())?,
            None => (None, None),
        };

        let glob_matcher = if let Some(content) = ignore_contents {
            let root_path = root_path.unwrap_or(Path::new("."));
            let matcher = GlobMatcher::new(root_path, &content)?;
            Some(matcher)
        } else {
            None
        };

        Ok(FileObjectFilter {
            conditions: vec![Conditions {
                name_regex,
                min_size,
                max_size,
                min_mtime,
                max_mtime,
            }],
            glob_matcher,
        })
    }

    pub fn add_or_condition(&mut self, condition: Conditions) {
        self.conditions.push(condition);
    }

    pub fn add_ignore_patterns(
        &mut self,
        root_path: &Path,
        ignore_content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.glob_matcher = Some(GlobMatcher::new(root_path, ignore_content)?);
        Ok(())
    }

    pub fn matches(&self, file_object: &FileObject) -> bool {
        // Check if the file should be ignored
        if let Some(ref matcher) = self.glob_matcher {
            if !matcher.should_process(file_object) {
                log::debug!("Ignoring file: {}", file_object.name());
                return false;
            }
        }

        // If no conditions are specified, file can be included
        if self.conditions.is_empty() {
            return true;
        }
        // process all conditions
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
