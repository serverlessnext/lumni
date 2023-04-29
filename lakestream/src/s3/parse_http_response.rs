use std::collections::HashMap;

use serde::Deserialize;

use crate::utils::time::rfc3339_to_epoch;
use crate::{Config, FileObject, ObjectStore};

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

pub fn parse_bucket_objects(
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

pub fn parse_file_objects(
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

pub fn extract_continuation_token(body: &str) -> Option<String> {
    let list_bucket_result: Result<ListBucketResult, _> =
        serde_xml_rs::from_str(body);

    match list_bucket_result {
        Ok(result) => result.NextContinuationToken,
        Err(_) => None,
    }
}
