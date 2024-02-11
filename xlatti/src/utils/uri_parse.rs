use regex::Regex;

#[derive(Debug)]
pub struct ParsedUri {
    pub scheme: Option<String>,
    pub bucket: Option<String>,
    pub path: Option<String>,
}

impl ParsedUri {
    pub fn from_uri(uri: &str, append_slash: bool) -> ParsedUri {
        if uri.is_empty() {
            return ParsedUri {
                scheme: None,
                bucket: None,
                path: None,
            };
        }

        let re = Regex::new(r"^(?P<scheme>[a-z0-9]+)://").unwrap();
        let scheme_match = re.captures(uri);

        scheme_match.map_or_else(
            || {
                // uri has no scheme, assume LocalFsBucket
                let (bucket, path) = parse_uri_path(None, uri, append_slash);
                ParsedUri {
                    scheme: None,
                    bucket,
                    path,
                }
            },
            |scheme_captures| {
                let scheme = scheme_captures.name("scheme").unwrap().as_str();
                let uri_without_scheme = re.replace(uri, "").to_string();
                if uri_without_scheme.is_empty() {
                    ParsedUri {
                        scheme: Some(scheme.to_string()),
                        bucket: None,
                        path: None,
                    }
                } else {
                    let (bucket, path) = parse_uri_path(
                        Some(scheme),
                        &uri_without_scheme,
                        append_slash,
                    );
                    ParsedUri {
                        scheme: Some(scheme.to_string()),
                        bucket,
                        path,
                    }
                }
            },
        )
    }
}

fn parse_uri_path(
    scheme: Option<&str>,
    uri_path: &str,
    append_slash: bool,
) -> (Option<String>, Option<String>) {
    let cleaned_uri = uri_path.trim_end_matches('.');

    if cleaned_uri.is_empty() {
        return (Some(".".to_string()), None);
    }

    let is_absolute = cleaned_uri.starts_with('/');
    let mut parts = cleaned_uri.splitn(2, '/');
    let bucket = parts.next().map(|s| s.to_string());

    let path = parts.next().filter(|s| !s.is_empty()).map(|s| {
        let cleaned_path = s.replace("./", "");
        if cleaned_path.ends_with('/') {
            if append_slash {
                cleaned_path
            } else {
                cleaned_path.trim_end_matches('/').to_string()
            }
        } else if append_slash {
            format!("{}/", cleaned_path)
        } else {
            cleaned_path
        }
    });

    // If there is no path, treat the input as a path instead of a bucket
    // bucket is currenth path on LocalFs
    if scheme.is_none() && path.is_none() && bucket.is_some() {
        if append_slash {
            return (Some(".".to_string()), Some(format!("{}/", bucket.unwrap())));
        }
        else {
            return (Some(".".to_string()), bucket);
        }
    }

    if let Some(bucket) = bucket {
        let formatted_bucket = if is_absolute {
            format!("/{}", bucket)
        } else {
            bucket
        };
        return (Some(formatted_bucket), path);
    }

    (Some(".".to_string()), None)
}
