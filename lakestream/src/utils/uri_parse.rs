use regex::Regex;

pub struct ParsedUri {
    pub scheme: Option<String>,
    pub bucket: Option<String>,
    pub prefix: Option<String>,
}

impl ParsedUri {
    pub fn from_uri(uri: &str) -> ParsedUri {
        if uri.is_empty() {
            return ParsedUri {
                scheme: None,
                bucket: None,
                prefix: None,
            };
        }

        let re = Regex::new(r"^(?P<scheme>[a-z0-9]+)://").unwrap();
        let scheme_match = re.captures(uri);

        scheme_match.map_or_else(
            || {
                // uri has no scheme, assume LocalFsBucket
                let (bucket, prefix) = parse_uri_path(uri);
                ParsedUri {
                    scheme: None,
                    bucket,
                    prefix,
                }
            },
            |scheme_captures| {
                let scheme = scheme_captures.name("scheme").unwrap().as_str();
                let uri_without_scheme = re.replace(uri, "").to_string();
                if uri_without_scheme.is_empty() {
                    ParsedUri {
                        scheme: Some(scheme.to_string()),
                        bucket: None,
                        prefix: None,
                    }
                } else {
                    let (bucket, prefix) = parse_uri_path(&uri_without_scheme);
                    ParsedUri {
                        scheme: Some(scheme.to_string()),
                        bucket,
                        prefix,
                    }
                }
            },
        )
    }
}

fn parse_uri_path(uri_path: &str) -> (Option<String>, Option<String>) {
    let cleaned_uri = uri_path.trim_end_matches('.');

    if cleaned_uri.is_empty() {
        return (Some(".".to_string()), None);
    }

    let is_absolute = cleaned_uri.starts_with('/');
    let mut parts = cleaned_uri.splitn(2, '/');
    let bucket = parts.next().map(|s| s.to_string());
    let prefix = parts.next().filter(|s| !s.is_empty()).map(|s| {
        let cleaned_prefix = s.replace("./", "");
        if cleaned_prefix.ends_with('/') {
            cleaned_prefix
        } else {
            format!("{}/", cleaned_prefix)
        }
    });

    if let Some(bucket) = bucket {
        let formatted_bucket = if is_absolute {
            format!("/{}", bucket)
        } else {
            bucket
        };
        return (Some(formatted_bucket), prefix);
    }

    (Some(".".to_string()), None)
}
