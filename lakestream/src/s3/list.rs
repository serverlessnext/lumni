use std::collections::HashMap;

use log::{error, info};
use serde::Deserialize;

use super::bucket::{get_endpoint_url, S3Bucket, S3Credentials};
use super::client::S3Client;
use crate::base::config::Config;
use crate::base::interfaces::ObjectStoreTrait;
use crate::http::requests::{http_get_request, http_get_request_with_headers};
use crate::utils::time::rfc3339_to_epoch;
use crate::{
    FileObject, FileObjectFilter, LakestreamError, ObjectStore,
    AWS_MAX_LIST_OBJECTS,
};

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ListAllMyBucketsResult {
    Buckets: Buckets,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Buckets {
    Bucket: Vec<Bucket>,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Bucket {
    Name: String,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ListBucketResult {
    Contents: Option<Vec<Content>>,
    CommonPrefixes: Option<Vec<CommonPrefix>>,
    NextContinuationToken: Option<String>,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Content {
    Key: String,
    LastModified: String,
    Size: u64,
    ETag: String,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct CommonPrefix {
    Prefix: String,
}

pub async fn list_files(
    s3_bucket: &S3Bucket,
    prefix: Option<&str>,
    recursive: bool,
    max_keys: Option<u32>,
    filter: &Option<FileObjectFilter>,
) -> Result<Vec<FileObject>, LakestreamError> {
    let access_key = s3_bucket
        .config()
        .get("AWS_ACCESS_KEY_ID")
        .expect("Missing access_key in the configuration");
    let secret_key = s3_bucket
        .config()
        .get("AWS_SECRET_ACCESS_KEY")
        .expect("Missing secret_key in the configuration");
    let mut region = s3_bucket
        .config()
        .get("AWS_REGION")
        .expect("AWS_REGION not set")
        .clone();

    let credentials =
        S3Credentials::new(String::from(access_key), String::from(secret_key));

    let mut all_file_objects = Vec::new();
    let body: String;

    let mut s3_client;
    let mut endpoint_url;

    let effective_max_keys = get_effective_max_keys(filter, max_keys);

    // get mutable config as it will be updated if there is a redirect
    let mut config = s3_bucket.config().clone();

    // Check for a 301 redirect and update the region if necessary
    loop {
        endpoint_url = get_endpoint_url(&config, Some(s3_bucket.name()));
        s3_client =
            S3Client::new(endpoint_url, region.clone(), credentials.clone());

        let headers = s3_client
            .generate_list_objects_headers(
                prefix,
                Some(effective_max_keys),
                None,
            )
            .unwrap();

        let result =
            http_get_request_with_headers(&s3_client.url(), &headers).await;

        match result {
            Ok((response_body, status, response_headers)) => {
                if status == 301 {
                    info!(
                        "Received a 301 redirect from S3 with status: {}",
                        status
                    );
                    if let Some(new_region) =
                        response_headers.get("x-amz-bucket-region")
                    {
                        region = new_region.to_owned();
                        config.insert("AWS_REGION".to_string(), region.clone());
                    } else {
                        error!(
                            "Error: Redirect without x-amz-bucket-region \
                             header"
                        );
                        return Ok(Vec::new());
                    }
                } else if response_body.is_empty() {
                    error!(
                        "Error: Received an empty response from S3 with \
                         status: {}",
                        status
                    );
                    return Ok(Vec::new());
                } else {
                    body = response_body;
                    break;
                }
            }
            Err(e) => {
                error!("Error in http_get_request: {}", e);
                return Ok(Vec::new());
            }
        }
    }

    let initial_file_objects = parse_file_objects(&body).unwrap_or_default();
    // Parse the file objects from the body and create two separate lists:
    // one for directories and one for non-directory file objects.
    let (initial_directories, mut filtered_initial_file_objects): (
        Vec<_>,
        Vec<_>,
    ) = initial_file_objects
        .into_iter()
        .partition(|file_object| file_object.name().ends_with('/'));

    // If filter is None, move directories into filtered_initial_file_objects
    if filter.is_none() {
        filtered_initial_file_objects
            .extend(initial_directories.iter().cloned());
    }

    // Extend all_file_objects with the filtered list of non-directory file objects.
    // If a filter is provided, apply it; otherwise, include all non-directory file objects.
    all_file_objects.extend(filtered_initial_file_objects.into_iter().filter(
        |file_object| filter.as_ref().map_or(true, |f| f.matches(file_object)),
    ));

    let continuation_token = extract_continuation_token(&body);

    // enter recursive lookup if max_keys not yet satisfied
    let remaining_keys =
        max_keys.map(|max| max.saturating_sub(all_file_objects.len() as u32));

    if remaining_keys.map_or(true, |keys| keys > 0) {
        if continuation_token.is_some() {
            all_file_objects.extend(
                list_files_next(
                    s3_bucket,
                    prefix.map(|p| p.to_owned()),
                    remaining_keys,
                    &mut s3_client,
                    continuation_token,
                    recursive,
                    &(*filter).clone(),
                )
                .await
                .map_err(LakestreamError::from)?,
            );
        }

        // If the recursive flag is set, process each subdirectory from current base level
        if recursive {
            // For each directory, create a new prefix by appending the directory name
            // to the current prefix and call list_files_next() to process the subdirectory.
            for directory in initial_directories
                .into_iter()
                .map(|file_object| file_object.name().to_owned())
            {
                let subdir_prefix =
                    Some(format!("{}{}", prefix.unwrap_or(""), directory));
                all_file_objects.extend(
                    list_files_next(
                        s3_bucket,
                        subdir_prefix,
                        max_keys.map(|max| max - all_file_objects.len() as u32),
                        &mut s3_client,
                        None,
                        recursive,
                        filter,
                    )
                    .await
                    .map_err(LakestreamError::from)?,
                );
            }
        }
    }
    Ok(all_file_objects)
}

async fn list_files_next(
    _s3_bucket: &S3Bucket,
    prefix: Option<String>,
    max_keys: Option<u32>,
    s3_client: &mut S3Client,
    continuation_token: Option<String>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
) -> Result<Vec<FileObject>, LakestreamError> {
    let mut all_file_objects = Vec::new();
    let mut virtual_directories = Vec::new();
    let mut current_continuation_token = continuation_token;
    let mut directory_stack = std::collections::VecDeque::new();

    directory_stack.push_back(prefix);

    let effective_max_keys = get_effective_max_keys(filter, max_keys);

    while let Some(prefix) = directory_stack.pop_front() {
        // loop until we have all regular files or we have reached the max_keys
        loop {
            let headers = s3_client
                .generate_list_objects_headers(
                    prefix.as_deref(),
                    Some(effective_max_keys),
                    current_continuation_token.as_deref(),
                )
                .unwrap();

            let result = http_get_request(&s3_client.url(), &headers).await;

            let (response_body, _) = match result {
                Ok(res) => res,
                Err(e) => return Err(LakestreamError::from(e)),
            };

            if !response_body.is_empty() {
                let file_objects =
                    parse_file_objects(&response_body).unwrap_or_default();

                for file_object in file_objects {
                    if all_file_objects.len()
                        == max_keys.unwrap_or(AWS_MAX_LIST_OBJECTS) as usize
                    {
                        break;
                    }

                    if file_object.name().ends_with('/') {
                        if recursive {
                            virtual_directories
                                .push(file_object.name().to_owned());
                        }
                        if filter.is_none() {
                            all_file_objects.push(file_object.clone());
                        }
                    } else {
                        // Check if the file_object satisfies the filter conditions
                        if let Some(ref filter) = filter {
                            if !filter.matches(&file_object) {
                                continue;
                            }
                        }
                        all_file_objects.push(file_object);
                    }
                }

                current_continuation_token =
                    extract_continuation_token(&response_body);
            }

            if current_continuation_token.is_none()
                || all_file_objects.len()
                    >= max_keys.unwrap_or(AWS_MAX_LIST_OBJECTS) as usize
            {
                break;
            }
        }

        if recursive {
            // if recursive is True, and we have not reached the max_keys,
            // continue with the virtual directories
            for virtual_directory in virtual_directories.drain(..) {
                if all_file_objects.len()
                    == max_keys.unwrap_or(AWS_MAX_LIST_OBJECTS) as usize
                {
                    break;
                }

                directory_stack.push_back(Some(virtual_directory));
            }
        }

        current_continuation_token = None;
    }

    Ok(all_file_objects)
}

fn extract_continuation_token(body: &str) -> Option<String> {
    let list_bucket_result: Result<ListBucketResult, _> =
        serde_xml_rs::from_str(body);

    match list_bucket_result {
        Ok(result) => result.NextContinuationToken,
        Err(_) => None,
    }
}

pub async fn list_buckets(
    config: &Config,
) -> Result<Vec<ObjectStore>, LakestreamError> {
    let region = config
        .get("AWS_REGION")
        .expect("Missing region in the configuration");
    let access_key = config
        .get("AWS_ACCESS_KEY_ID")
        .expect("Missing access_key in the configuration");
    let secret_key = config
        .get("AWS_SECRET_ACCESS_KEY")
        .expect("Missing secret_key in the configuration");

    let credentials =
        S3Credentials::new(String::from(access_key), String::from(secret_key));
    let endpoint_url = get_endpoint_url(config, None);

    let mut s3_client =
        S3Client::new(endpoint_url, String::from(region), credentials);

    let headers: HashMap<String, String> =
        s3_client.generate_list_buckets_headers().unwrap();
    let result = http_get_request(&s3_client.url().clone(), &headers).await;

    let bucket_objects = match result {
        Ok((body, _)) => {
            match parse_bucket_objects(&body, Some(config.clone())) {
                Ok(bucket_objects) => bucket_objects,
                Err(e) => {
                    error!("Error listing bucket objects: {}", e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            error!("Error in http_get_request: {}", e);
            Vec::new()
        }
    };

    Ok(bucket_objects)
}

fn parse_bucket_objects(
    body: &str,
    config: Option<Config>,
) -> Result<Vec<ObjectStore>, Box<dyn std::error::Error>> {
    let list_all_my_buckets_result: ListAllMyBucketsResult =
        serde_xml_rs::from_str(body)?;
    let object_stores: Vec<ObjectStore> = list_all_my_buckets_result
        .Buckets
        .Bucket
        .iter()
        .map(|bucket| {
            let name = bucket.Name.clone();
            let config = config.clone().unwrap_or_default();
            ObjectStore::new(&format!("s3://{}", name), config).unwrap()
        })
        .collect();
    Ok(object_stores)
}

fn parse_file_objects(
    body: &str,
) -> Result<Vec<FileObject>, Box<dyn std::error::Error>> {
    let list_bucket_result: ListBucketResult = serde_xml_rs::from_str(body)?;
    let file_objects: Vec<FileObject> = list_bucket_result
        .Contents
        .unwrap_or_default()
        .iter()
        .map(|content| {
            FileObject::new(
                content.Key.clone(),
                content.Size,
                Some(rfc3339_to_epoch(content.LastModified.as_str()).unwrap()),
                Some(
                    [(
                        "ETag".to_string(),
                        content.ETag.trim_matches('"').to_string(),
                    )]
                    .iter()
                    .cloned()
                    .collect::<HashMap<String, String>>(),
                ),
            )
        })
        .collect();
    let common_prefixes: Vec<String> = list_bucket_result
        .CommonPrefixes
        .unwrap_or_default()
        .iter()
        .map(|common_prefix| common_prefix.Prefix.clone())
        .collect();
    let common_prefix_file_objects: Vec<FileObject> = common_prefixes
        .iter()
        .map(|prefix| {
            FileObject::new(
                prefix.clone(), // Set the key to the prefix
                0,              // Set the size to 0
                None,           // Set the modified timestamp to None
                None,           // Set the tags to None
            )
        })
        .collect();
    let all_file_objects: Vec<FileObject> =
        [&file_objects[..], &common_prefix_file_objects[..]].concat();
    Ok(all_file_objects)
}

fn get_effective_max_keys(
    filter: &Option<FileObjectFilter>,
    max_keys: Option<u32>,
) -> u32 {
    if filter.is_some() {
        AWS_MAX_LIST_OBJECTS
    } else {
        max_keys.unwrap_or(AWS_MAX_LIST_OBJECTS)
    }
}
