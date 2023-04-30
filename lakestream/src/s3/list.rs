use std::collections::HashMap;

use log::{error, info};

use super::bucket::{configure_bucket_url, S3Bucket, S3Credentials};
use super::client::{S3Client, S3ClientConfig};
use super::parse_http_response::{
    extract_continuation_token, parse_bucket_objects, parse_file_objects,
};
use crate::base::config::Config;
use crate::base::interfaces::ObjectStoreTrait;
use crate::http::requests::{http_get_request, http_get_request_with_headers};
use crate::{
    FileObjectFilter, FileObjectVec, LakestreamError, ObjectStore,
    AWS_MAX_LIST_OBJECTS,
};

pub async fn list_files(
    s3_bucket: &S3Bucket,
    prefix: Option<&str>,
    recursive: bool,
    max_keys: Option<u32>,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut FileObjectVec,
) -> Result<(), LakestreamError> {
    let mut s3_client =
        create_s3_client(s3_bucket.config(), Some(s3_bucket.name()));

    let mut bucket_url;
    let body: String;

    let effective_max_keys = get_effective_max_keys(filter, max_keys);

    // get mutable config as it will be updated if there is a redirect
    let mut config = s3_bucket.config().clone();
    let mut region: String = s3_client.region().to_string();

    // Check for a 301 redirect and update the region if necessary
    loop {
        bucket_url = configure_bucket_url(&config, Some(s3_bucket.name()));
        let credentials = s3_client.credentials().clone();
        let s3_client_config =
            S3ClientConfig::new(credentials, &bucket_url, &region);
        s3_client = S3Client::new(s3_client_config);

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
                        region = new_region.clone();
                        config.insert(
                            "AWS_REGION".to_string(),
                            region.to_string(),
                        );
                    } else {
                        error!(
                            "Error: Redirect without x-amz-bucket-region \
                             header"
                        );
                        return Ok(());
                    }
                } else if response_body.is_empty() {
                    error!(
                        "Error: Received an empty response from S3 with \
                         status: {}",
                        status
                    );
                    return Ok(());
                } else {
                    body = response_body;
                    break;
                }
            }
            Err(e) => {
                error!("Error in http_get_request: {}", e);
                return Ok(());
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
    file_objects.extend(filtered_initial_file_objects.into_iter().filter(
        |file_object| filter.as_ref().map_or(true, |f| f.matches(file_object)),
    ));

    let continuation_token = extract_continuation_token(&body);

    // enter recursive lookup if max_keys not yet satisfied
    let remaining_keys =
        max_keys.map(|max| max.saturating_sub(file_objects.len() as u32));

    if remaining_keys.map_or(true, |keys| keys > 0) {
        if continuation_token.is_some() {
            list_files_next(
                s3_bucket,
                prefix.map(|p| p.to_owned()),
                remaining_keys,
                &mut s3_client,
                continuation_token,
                recursive,
                &(*filter).clone(),
                file_objects,
            )
            .await?;
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

                list_files_next(
                    s3_bucket,
                    subdir_prefix,
                    remaining_keys,
                    &mut s3_client,
                    None,
                    recursive,
                    &(*filter).clone(),
                    file_objects,
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn list_files_next(
    _s3_bucket: &S3Bucket,
    prefix: Option<String>,
    max_keys: Option<u32>,
    s3_client: &mut S3Client,
    continuation_token: Option<String>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut FileObjectVec,
) -> Result<(), LakestreamError> {
    let mut virtual_directories = Vec::new();
    let mut current_continuation_token = continuation_token;
    let mut directory_stack = std::collections::VecDeque::new();

    directory_stack.push_back(prefix);

    let effective_max_keys = get_effective_max_keys(filter, max_keys);

    while let Some(prefix) = directory_stack.pop_front() {
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
                let file_objects_list =
                    parse_file_objects(&response_body).unwrap_or_default();

                for file_object in file_objects_list {
                    if file_objects.len()
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
                            file_objects.push(file_object.clone());
                        }
                    } else {
                        if let Some(ref filter) = filter {
                            if !filter.matches(&file_object) {
                                continue;
                            }
                        }
                        file_objects.push(file_object);
                    }
                }

                current_continuation_token =
                    extract_continuation_token(&response_body);
            }

            if current_continuation_token.is_none()
                || file_objects.len()
                    >= max_keys.unwrap_or(AWS_MAX_LIST_OBJECTS) as usize
            {
                break;
            }
        }

        if recursive {
            for virtual_directory in virtual_directories.drain(..) {
                if file_objects.len()
                    == max_keys.unwrap_or(AWS_MAX_LIST_OBJECTS) as usize
                {
                    break;
                }

                directory_stack.push_back(Some(virtual_directory));
            }
        }

        current_continuation_token = None;
    }

    Ok(())
}

pub async fn list_buckets(
    config: &Config,
) -> Result<Vec<ObjectStore>, LakestreamError> {
    let mut s3_client = create_s3_client(config, None);

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

fn create_s3_client(config: &Config, bucket_name: Option<&str>) -> S3Client {
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
    let bucket_url = configure_bucket_url(config, bucket_name);

    let s3_client_config =
        S3ClientConfig::new(credentials, &bucket_url, region);
    S3Client::new(s3_client_config)
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
