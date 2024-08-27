use std::path::Path;

use regex::Regex;

use super::glob_matcher::GlobMatcher;
use super::ignore_contents::IgnoreContents;
use super::{Conditions, ParseFilterCondition};
use crate::utils::time::Timestamp;
use crate::{FileObject, LumniError};

#[derive(Debug, Clone)]
pub struct FileObjectFilter {
    pub conditions: Vec<Conditions>,
    pub glob_matcher: Option<GlobMatcher>,
    pub include_directories: bool,
}

impl FileObjectFilter {
    pub fn new(conditions: Conditions, include_directories: bool) -> Self {
        // include directories if no conditions are specified
        FileObjectFilter {
            conditions: if conditions.is_empty() {
                Vec::new()
            } else {
                vec![conditions]
            },
            glob_matcher: None,
            include_directories,
        }
    }

    pub fn add_glob_matcher(&mut self, glob_matcher: GlobMatcher) {
        self.glob_matcher = Some(glob_matcher);
    }

    pub fn new_with_single_condition(
        name: Option<&str>,
        size: Option<&str>,
        mtime: Option<&str>,
        ignore_contents: Option<IgnoreContents>,
        include_directories: bool,
    ) -> Result<Self, LumniError> {
        let name_regex = name.map(|pattern| Regex::new(pattern).unwrap());

        let (min_size, max_size) = match size {
            Some(s) => ParseFilterCondition::size(s)?,
            None => (None, None),
        };

        let system_time_seconds = Timestamp::from_system_time()?.as_seconds();
        let (min_mtime, max_mtime) = match mtime {
            Some(m) => ParseFilterCondition::time(m, system_time_seconds)?,
            None => (None, None),
        };

        let glob_matcher = if let Some(ignore_contents) = ignore_contents {
            ignore_contents.to_glob_matcher()?
        } else {
            None
        };

        let conditions = Conditions {
            name_regex,
            min_size,
            max_size,
            min_mtime,
            max_mtime,
        };

        Ok(FileObjectFilter {
            conditions: if conditions.is_empty() {
                Vec::new()
            } else {
                vec![conditions]
            },
            glob_matcher,
            include_directories,
        })
    }

    pub fn add_or_condition(&mut self, condition: Conditions) {
        if self.include_directories && !condition.is_empty() {
            self.include_directories = false;
        }
        self.conditions.push(condition);
    }

    pub fn add_ignore_patterns(
        &mut self,
        root_path: &Path,
        ignore_content: &str,
    ) -> Result<(), LumniError> {
        self.glob_matcher = Some(GlobMatcher::new(root_path, ignore_content)?);
        Ok(())
    }

    pub fn ignore_matches(&self, path: &Path) -> bool {
        if let Some(ref matcher) = self.glob_matcher {
            return matcher.should_ignore(path);
        }
        false
    }

    pub fn condition_matches(&self, file_object: &FileObject) -> bool {
        // if file_object is a directory and directories are not included, return false directly
        if !self.include_directories && file_object.is_directory() {
            return false;
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
