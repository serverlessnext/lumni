use std::fs;
use std::path::Path;

use super::glob_matcher::GlobMatcher;
use crate::LumniError;

pub struct IgnoreContents {
    root_path: Option<&'static Path>,
    contents: Option<String>,
}

impl IgnoreContents {
    pub fn new(
        ignore_files: Vec<String>,
        add_gitignore: bool,
    ) -> IgnoreContents {
        // Check if '.gitignore' is already included in the user-specified files
        let gitignore_included =
            ignore_files.iter().any(|file| file == ".gitignore");

        // Include '.gitignore' by default unless no_gitignore is set or it's already included
        let ignore_files = if add_gitignore && !gitignore_included {
            vec![".gitignore".to_string()]
                .into_iter()
                .chain(ignore_files.into_iter())
                .collect()
        } else {
            ignore_files
        };

        let (root_path, contents) =
            IgnoreContents::parse_ignore_files(&ignore_files);
        IgnoreContents {
            root_path,
            contents,
        }
    }

    pub fn to_glob_matcher(&self) -> Result<Option<GlobMatcher>, LumniError> {
        if let Some(ignore_contents) = &self.contents {
            let root_path = self.get_root_path();
            let matcher = GlobMatcher::new(root_path, ignore_contents)?;
            Ok(Some(matcher))
        } else {
            Ok(None)
        }
    }

    fn get_root_path(&self) -> &'static Path {
        self.root_path.unwrap_or(Path::new("."))
    }

    pub fn get_contents(&self) -> Option<&String> {
        self.contents.as_ref()
    }

    fn parse_ignore_files(
        ignore_files: &Vec<String>,
    ) -> (Option<&'static Path>, Option<String>) {
        let mut contents = String::new();
        for ignore_file in ignore_files {
            let gitignore_path = Path::new(ignore_file);
            if gitignore_path.exists() {
                match fs::read_to_string(gitignore_path) {
                    Ok(content) => contents.push_str(&content),
                    Err(error) => {
                        log::error!("Error reading ignore file: {}", error)
                    }
                }
            }
        }
        if !contents.is_empty() {
            let root_path = Path::new(".");
            return (Some(root_path), Some(contents)); // Successfully aggregated the contents
        }
        (None, None)
    }
}
