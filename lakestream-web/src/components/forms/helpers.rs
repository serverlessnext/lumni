
use regex::Regex;

pub fn validate_with_pattern(
    pattern: Regex,
    error_msg: String,
) -> Box<dyn Fn(&str) -> Result<(), String>> {
    Box::new(move |input: &str| {
        if pattern.is_match(input) {
            Ok(())
        } else {
            Err(error_msg.clone())
        }
    })
}
