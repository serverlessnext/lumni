use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};

use crate::FileObject;

#[derive(Debug, Clone)]
pub struct GlobMatcher {
    ignore_set: GlobSet,
    whitelist_set: GlobSet,
    root_path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum MatchResult {
    Ignore,
    Whitelist,
    None,
}

impl GlobMatcher {
    pub fn new(root_path: &Path, ignore_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ignore_builder = GlobSetBuilder::new();
        let mut whitelist_builder = GlobSetBuilder::new();
        
        for line in ignore_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            let (is_whitelist, pattern) = if line.starts_with('!') {
                (true, &line[1..])
            } else {
                (false, line)
            };
            
            let pattern = if pattern.starts_with('/') {
                format!("{}**", pattern)
            } else {
                format!("**/{}", pattern)
            };
            
            let glob = Glob::new(&pattern)?;
            if is_whitelist {
                whitelist_builder.add(glob);
            } else {
                ignore_builder.add(glob);
            }
        }
        
        Ok(Self {
            ignore_set: ignore_builder.build()?,
            whitelist_set: whitelist_builder.build()?,
            root_path: root_path.to_path_buf(),
        })
    }

    fn match_path(&self, path: &Path) -> MatchResult {
        if self.whitelist_set.is_match(path) {
            MatchResult::Whitelist
        } else if self.ignore_set.is_match(path) {
            MatchResult::Ignore
        } else {
            MatchResult::None
        }
    }

    pub fn should_process(&self, file_object: &FileObject) -> bool {
        let file_path = Path::new(file_object.name());
        let relative_path = file_path.strip_prefix(&self.root_path).unwrap_or(file_path);
        let is_dir = file_object.name().ends_with('/');

        // First, check if the path or any of its ancestors are whitelisted
        for ancestor in relative_path.ancestors() {
            if self.whitelist_set.is_match(ancestor) {
                return true;
            }
        }

        // Then, check if the path or any of its ancestors are ignored
        for ancestor in relative_path.ancestors() {
            if self.ignore_set.is_match(ancestor) {
                // For directory patterns, only ignore if it's actually a directory
                if is_dir || !ancestor.to_str().unwrap_or("").ends_with('/') {
                    return false;
                }
            }
        }

        // If we've reached here, the path is neither whitelisted nor ignored
        true
    }
}
