mod file_object_filter;
mod glob_matcher;
mod parse_filter_condition;
mod parse_where_clause;

pub use file_object_filter::FileObjectFilter;
pub use parse_filter_condition::ParseFilterCondition;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Conditions {
    pub name_regex: Option<Regex>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_mtime: Option<u64>,
    pub max_mtime: Option<u64>,
}
