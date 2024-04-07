use std::collections::HashMap;

use regex::Regex;


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
