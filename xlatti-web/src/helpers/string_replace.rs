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
