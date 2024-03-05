use std::collections::HashMap;

use regex::Regex;

pub fn replace_first_single_colon(s: &str) -> String {
    if let Some(pos) = s.find(':') {
        if pos == s.len() - 1 || s.as_bytes()[pos + 1] != b':' {
            let mut result = s.to_string();
            result.replace_range(pos..pos + 1, "/");
            return result;
        }
    }
    s.to_string()
}

pub fn replace_variables_in_string_with_map(
    query: &str,
    vars_map: &HashMap<String, String>,
) -> String {
    // replace variables in a string based on a provided hashmap
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    re.replace_all(query, |caps: &regex::Captures| {
        vars_map
            .get(&caps[1])
            .unwrap_or(&"".to_string())
            .to_string()
    })
    .to_string()
}
