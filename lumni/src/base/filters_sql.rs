use regex::Regex;
use sqlparser::ast::{BinaryOperator, Expr, Value};

use super::filters::{parse_size, Conditions, FileObjectFilter};
use crate::LakestreamError;

impl FileObjectFilter {
    pub fn parse_where_clause(
        where_clause: &Expr,
    ) -> Result<Option<FileObjectFilter>, LakestreamError> {
        match where_clause {
            Expr::BinaryOp { left, op, right } => match op {
                BinaryOperator::Or => {
                    let mut left_filter =
                        parse_condition(left)?.ok_or_else(|| {
                            LakestreamError::InternalError(
                                "Invalid left condition in OR".to_string(),
                            )
                        })?;

                    if let Some(right_conditions) = parse_condition(right)? {
                        for condition in right_conditions.conditions {
                            left_filter.add_or_condition(condition);
                        }
                    }
                    Ok(Some(left_filter))
                }
                _ => parse_condition(where_clause),
            },
            _ => parse_condition(where_clause),
        }
    }

    pub fn from_sql_conditions(
        name_regex: Option<Regex>,
        size: Option<&str>,
        mtime: Option<&str>,
    ) -> Result<Self, String> {
        log::debug!(
            "SQL conditions - name_regex: {:?}, size: {:?}, mtime: {:?}",
            name_regex, size, mtime
        );
        let (min_size, max_size) = match size {
            Some(s) => parse_size(s)?,
            None => (None, None),
        };

        let (min_mtime, max_mtime) = match mtime {
            Some(m) => parse_absolute_time_condition(m)?,
            None => (None, None),
        };

        Ok(FileObjectFilter::new(Conditions {
            name_regex,
            min_size,
            max_size,
            min_mtime,
            max_mtime,
        }))
    }
}

fn parse_condition(
    expr: &Expr,
) -> Result<Option<FileObjectFilter>, LakestreamError> {
    match expr {
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOperator::And => parse_and_condition(left, right),
            _ => parse_single_condition(left, op, right),
        },
        Expr::Like { expr, pattern, .. } => parse_like_condition(expr, pattern),
        _ => {
            Err(LakestreamError::InternalError(
                "Unsupported WHERE clause structure".to_string(),
            ))
        }
    }
}

fn parse_and_condition(
    left: &Expr,
    right: &Expr,
) -> Result<Option<FileObjectFilter>, LakestreamError> {
    let left_filter = parse_condition(left)?;
    let right_filter = parse_condition(right)?;

    match (left_filter, right_filter) {
        (Some(left), Some(right)) => {
            let mut combined_filter = FileObjectFilter {
                conditions: Vec::new(),
            };

            for left_condition in &left.conditions {
                for right_condition in &right.conditions {
                    let combined_conditions = Conditions {
                        name_regex: left_condition
                            .name_regex
                            .clone()
                            .or(right_condition.name_regex.clone()),
                        min_size: max_option(
                            left_condition.min_size,
                            right_condition.min_size,
                        ),
                        max_size: min_option(
                            left_condition.max_size,
                            right_condition.max_size,
                        ),
                        min_mtime: max_option(
                            left_condition.min_mtime,
                            right_condition.min_mtime,
                        ),
                        max_mtime: min_option(
                            left_condition.max_mtime,
                            right_condition.max_mtime,
                        ),
                    };
                    combined_filter.add_or_condition(combined_conditions);
                }
            }
            Ok(Some(combined_filter))
        }
        (left, right) => Ok(left.or(right)),
    }
}

fn max_option<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

fn min_option<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

fn parse_single_condition(
    left: &Expr,
    op: &BinaryOperator,
    right: &Expr,
) -> Result<Option<FileObjectFilter>, LakestreamError> {
    if let Expr::Identifier(ident) = left {
        match ident.value.as_str() {
            "name" => parse_name_condition(op, right),
            "size" => parse_size_condition(op, right),
            "mtime" => parse_mtime_condition(op, right),
            _ => {
                Err(LakestreamError::InternalError(format!(
                    "Unsupported field in WHERE clause: {}",
                    ident.value
                )))
            }
        }
    } else {
        Err(LakestreamError::InternalError(
            "Unexpected left-hand expression in WHERE clause".to_string(),
        ))
    }
}

fn parse_like_condition(
    expr: &Expr,
    pattern: &Expr,
) -> Result<Option<FileObjectFilter>, LakestreamError> {
    if let Expr::Identifier(ident) = expr {
        if ident.value == "name" {
            match pattern {
                Expr::Value(Value::SingleQuotedString(s))
                | Expr::Value(Value::DoubleQuotedString(s)) => {
                    let pattern = sql_like_to_regex(s);
                    Ok(Some(FileObjectFilter::from_sql_conditions(
                        Some(pattern),
                        None,
                        None,
                    )?))
                }
                Expr::Identifier(ident) => {
                    let pattern = sql_like_to_regex(&ident.value);
                    Ok(Some(FileObjectFilter::from_sql_conditions(
                        Some(pattern),
                        None,
                        None,
                    )?))
                }
                _ => Err(LakestreamError::InternalError(format!(
                    "Unsupported pattern type for LIKE operation: {:?}",
                    pattern
                ))),
            }
        } else {
            Err(LakestreamError::InternalError(format!(
                "LIKE operation not supported for field: {}",
                ident.value
            )))
        }
    } else {
        Err(LakestreamError::InternalError(
            "Unexpected expression in LIKE operation".to_string(),
        ))
    }
}

fn sql_like_to_regex(like_pattern: &str) -> Regex {
    let mut regex_pattern = String::with_capacity(like_pattern.len() * 2);
    regex_pattern.push('^');

    for c in like_pattern.chars() {
        match c {
            '%' => regex_pattern.push_str(".*"),
            '_' => regex_pattern.push('.'),
            '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{'
            | '}' | '|' | '\\' => {
                regex_pattern.push('\\');
                regex_pattern.push(c);
            }
            _ => regex_pattern.push(c),
        }
    }

    regex_pattern.push('$');
    Regex::new(&regex_pattern).unwrap()
}

fn parse_name_condition(
    op: &BinaryOperator,
    right: &Expr,
) -> Result<Option<FileObjectFilter>, LakestreamError> {
    let value = match right {
        Expr::Value(Value::SingleQuotedString(s))
        | Expr::Value(Value::DoubleQuotedString(s)) => s,
        Expr::Identifier(ident) => &ident.value,
        _ => {
            return Err(LakestreamError::InternalError(
                "Unsupported value type for name comparison".to_string(),
            ));
        }
    };
    match op {
        BinaryOperator::Eq | BinaryOperator::PGLikeMatch => {
            let pattern = sql_like_to_regex(value);
            Ok(Some(FileObjectFilter::from_sql_conditions(
                Some(pattern),
                None,
                None,
            )?))
        }
        _ => {
            Err(LakestreamError::InternalError(
                "Unsupported operator for name comparison".to_string(),
            ))
        }
    }
}

fn parse_size_condition(
    op: &BinaryOperator,
    right: &Expr,
) -> Result<Option<FileObjectFilter>, LakestreamError> {
    if let Expr::Value(Value::Number(n, _)) = right {
        let condition = match op {
            BinaryOperator::Gt => format!(">{}", n),
            BinaryOperator::Lt => format!("<{}", n),
            BinaryOperator::GtEq => format!(">={}", n),
            BinaryOperator::LtEq => format!("<={}", n),
            BinaryOperator::Eq => format!("={}", n),
            _ => {
                return Err(LakestreamError::InternalError(
                    "Unsupported operator for size comparison".to_string(),
                ))
            }
        };
        Ok(Some(FileObjectFilter::from_sql_conditions(
            None,
            Some(&condition),
            None,
        )?))
    } else {
        Err(LakestreamError::InternalError(
            "Invalid value type for size comparison".to_string(),
        ))
    }
}

fn parse_mtime_condition(
    op: &BinaryOperator,
    right: &Expr,
) -> Result<Option<FileObjectFilter>, LakestreamError> {
    if let Expr::Value(Value::Number(n, _)) = right {
        let condition = match op {
            BinaryOperator::Gt => format!(">{}", n),
            BinaryOperator::Lt => format!("<{}", n),
            BinaryOperator::GtEq => format!(">={}", n),
            BinaryOperator::LtEq => format!("<={}", n),
            BinaryOperator::Eq => format!("={}", n),
            _ => {
                return Err(LakestreamError::InternalError(
                    "Unsupported operator for mtime comparison".to_string(),
                ))
            }
        };
        Ok(Some(FileObjectFilter::from_sql_conditions(
            None,
            None,
            Some(&condition),
        )?))
    } else {
        Err(LakestreamError::InternalError(
            "Invalid value type for mtime comparison".to_string(),
        ))
    }
}

fn parse_absolute_time_condition(
    time_str: &str,
) -> Result<(Option<u64>, Option<u64>), String> {
    if time_str.contains("..") {
        // Handle range
        let parts: Vec<&str> = time_str.split("..").collect();
        if parts.len() != 2 {
            return Err(format!("Invalid time range format: {}", time_str));
        }
        let start = if parts[0].is_empty() {
            None
        } else {
            Some(
                parts[0]
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid start time: {}", parts[0]))?,
            )
        };
        let end = if parts[1].is_empty() {
            None
        } else {
            Some(
                parts[1]
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid end time: {}", parts[1]))?,
            )
        };
        return Ok((start, end));
    }

    match time_str.chars().next() {
        Some('>') => {
            if time_str.chars().nth(1) == Some('=') {
                let timestamp = time_str[2..].parse::<u64>().map_err(|_| {
                    format!("Invalid time format: {}", time_str)
                })?;
                Ok((Some(timestamp), None))
            } else {
                let timestamp = time_str[1..].parse::<u64>().map_err(|_| {
                    format!("Invalid time format: {}", time_str)
                })?;
                Ok((Some(timestamp + 1), None)) // Exclusive lower bound
            }
        }
        Some('<') => {
            if time_str.chars().nth(1) == Some('=') {
                let timestamp = time_str[2..].parse::<u64>().map_err(|_| {
                    format!("Invalid time format: {}", time_str)
                })?;
                Ok((None, Some(timestamp))) // Inclusive upper bound
            } else {
                let timestamp = time_str[1..].parse::<u64>().map_err(|_| {
                    format!("Invalid time format: {}", time_str)
                })?;
                Ok((None, Some(timestamp.saturating_sub(1)))) // Exclusive upper bound, converted to inclusive
            }
        }
        Some('=') => {
            let timestamp = time_str[1..]
                .parse::<u64>()
                .map_err(|_| format!("Invalid time format: {}", time_str))?;
            Ok((Some(timestamp), Some(timestamp))) // Inclusive single point
        }
        _ => {
            // Try parsing as a direct timestamp if no prefix is found
            let timestamp = time_str
                .parse::<u64>()
                .map_err(|_| format!("Invalid time format: {}", time_str))?;
            Ok((Some(timestamp), Some(timestamp))) // Inclusive single point
        }
    }
}
// test cases
// SELECT * FROM "localfs://." WHERE name = "Makefile" LIMIT 5
// SELECT * FROM "localfs://." WHERE name LIKE "%.txt" LIMIT 10
// SELECT * FROM "localfs://." WHERE size > 1000000 LIMIT 5
// SELECT * FROM "localfs://." WHERE mtime > 1625097600 LIMIT 5
// SELECT * FROM "localfs://." WHERE name LIKE "%.log" AND size > 500000 LIMIT 5
// SELECT * FROM "localfs://." WHERE name = 'README.md' LIMIT 5
// SELECT * FROM "localfs://." WHERE size >= 1000000 AND size <= 5000000 LIMIT 10
// SELECT * FROM "localfs://." WHERE name LIKE "data%" AND mtime > 1609459200 LIMIT 5
// SELECT * FROM "localfs://." WHERE name LIKE "log_2023_%_error.txt" LIMIT 10
// SELECT * FROM "localfs://." WHERE name LIKE "%.csv" AND size > 1000000 AND mtime > 1640995200 LIMIT 5
