use super::time_parse_ext::epoch_to_rfc3339;

pub fn time_human_readable(epoch_time: i64) -> String {
    epoch_to_rfc3339(epoch_time).unwrap()
}

pub fn bytes_human_readable(size: u64) -> String {
    let exponent: u32;
    let symbol: char;

    if size < 1024 {
        return size.to_string();
    } else if size < 1024u64.pow(2) {
        exponent = 1;
        symbol = 'k';
    } else if size < 1024u64.pow(3) {
        exponent = 2;
        symbol = 'M';
    } else if size < 1024u64.pow(4) {
        exponent = 3;
        symbol = 'G';
    } else if size < 1024u64.pow(5) {
        exponent = 4;
        symbol = 'T';
    } else if size < 1024u64.pow(6) {
        exponent = 5;
        symbol = 'P';
    } else if size < 1024u64.pow(7) {
        exponent = 6;
        symbol = 'E';
    } else if size < 1024u64.pow(8) {
        exponent = 7;
        symbol = 'Z';
    } else {
        return "Inf".to_string();
    }

    format!(
        "{:.1}{}",
        (size as f64 / 1024f64.powi(exponent as i32)),
        symbol
    )
}
