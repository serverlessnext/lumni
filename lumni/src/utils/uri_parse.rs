use regex::Regex;


#[derive(Debug, PartialEq)]
pub enum UriScheme {
    LocalFs,
    S3,
    Http,
    Https,
    None,
    Unsupported(String)
}

impl UriScheme {
    pub fn from_str(scheme: &str) -> Self {
        match scheme {
            "localfs" => UriScheme::LocalFs,
            "s3" => UriScheme::S3,
            "http" => UriScheme::Http,
            "https" => UriScheme::Https,
            "" => UriScheme::None,
            _ => UriScheme::Unsupported(scheme.to_string()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            UriScheme::LocalFs => "localfs".to_string(),
            UriScheme::S3 => "s3".to_string(),
            UriScheme::Http => "http".to_string(),
            UriScheme::Https => "https".to_string(),
            UriScheme::None => "".to_string(),
            UriScheme::Unsupported(scheme) => scheme.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ParsedUri {
    pub scheme: UriScheme,
    pub bucket: Option<String>,
    pub path: Option<String>,
}

impl ParsedUri {
    pub fn to_string(&self) -> String {
        let mut uri = self.scheme.to_string();
        if !uri.is_empty() {
            uri.push_str("://");
        }

        if let Some(bucket) = &self.bucket {
            uri.push_str(&bucket);
        }

        if let Some(path) = &self.path {
            uri.push_str(&path);
        }
        uri
    }

    pub fn from_uri(uri: &str, append_slash: bool) -> ParsedUri {
        if uri.is_empty() {
            return ParsedUri {
                scheme: UriScheme::None,
                bucket: None,
                path: None,
            };
        }

        let re = Regex::new(r"^(?P<scheme>[a-z0-9]+)://").unwrap();
        let scheme_match = re.captures(uri);

        scheme_match.map_or_else(
            || {
                // uri has no scheme, assume http://
                let uri_scheme = UriScheme::Http;
                let (bucket, path) = parse_uri_path(&uri_scheme, uri, append_slash);
                ParsedUri {
                    scheme: uri_scheme,
                    bucket,
                    path,
                }
            },
            |scheme_captures| {
                let scheme = scheme_captures.name("scheme").unwrap().as_str();
                let uri_without_scheme = re.replace(uri, "").to_string();
                if uri_without_scheme.is_empty() {
                    ParsedUri {
                        scheme: UriScheme::from_str(scheme),
                        bucket: None,
                        path: None,
                    }
                } else {
                    let uri_scheme = UriScheme::from_str(scheme);
                    let (bucket, path) = parse_uri_path(
                        &uri_scheme,
                        &uri_without_scheme,
                        append_slash,
                    );
                    ParsedUri {
                        scheme: uri_scheme,
                        bucket,
                        path,
                    }
                }
            },
        )
    }
}

fn parse_uri_path(
    scheme: &UriScheme,
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
    if scheme != &UriScheme::S3 && path.is_none() && bucket.is_some() { 
        if append_slash {
            return (
                Some(".".to_string()),
                Some(format!("{}/", bucket.unwrap())),
            );
        } else {
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
