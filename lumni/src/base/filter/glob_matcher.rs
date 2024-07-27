use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::api::error::LumniError;

#[derive(Debug, Clone)]
pub struct GlobMatcher {
    ignore_set: GlobSet,
    whitelist_set: GlobSet,
    root_path: PathBuf,
}

impl GlobMatcher {
    pub fn new(
        root_path: &Path,
        ignore_content: &str,
    ) -> Result<Self, LumniError> {
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
                pattern.to_string()
            } else {
                format!("**/{}", pattern)
            };

            // If the pattern ends with '/', append '**' to match all contents
            let pattern = if pattern.ends_with('/') {
                format!("{}**", pattern)
            } else {
                pattern
            };

            let glob = Glob::new(&pattern)
                .map_err(|e| LumniError::Any(e.to_string()))?;

            if is_whitelist {
                whitelist_builder.add(glob);
            } else {
                ignore_builder.add(glob);
            }
        }

        Ok(Self {
            ignore_set: ignore_builder
                .build()
                .map_err(|e| LumniError::Any(e.to_string()))?,
            whitelist_set: whitelist_builder
                .build()
                .map_err(|e| LumniError::Any(e.to_string()))?,
            root_path: root_path.to_path_buf(),
        })
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        let relative_path = path.strip_prefix(&self.root_path).unwrap_or(path);

        // First, check if the path or any of its ancestors are whitelisted
        for ancestor in relative_path.ancestors() {
            if self.whitelist_set.is_match(ancestor) {
                return false;
            }
        }

        // Check if the path or any of its ancestors are ignored
        for ancestor in relative_path.ancestors() {
            if self.ignore_set.is_match(ancestor) {
                return true;
            }
        }
        // Path is neither whitelisted nor ignored
        false
    }
}
