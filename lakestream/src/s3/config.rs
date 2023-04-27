use std::collections::HashMap;
use std::env;

use log::info;

use crate::LakestreamError;
use crate::AWS_DEFAULT_REGION;

pub fn update_config(
    config: &HashMap<String, String>,
) -> Result<HashMap<String, String>, LakestreamError> {
    let mut updated_config = config.clone();

    if updated_config.contains_key("AWS_REGION") {
        // AWS_REGION is defined in updated_config, do nothing.
    } else if let Some(region) = updated_config.get("region").cloned() {
        // "region" key is defined in updated_config, copy its value and re-insert with AWS_REGION.
        updated_config.insert("AWS_REGION".to_string(), region);
    } else {
        // "region" key is not defined in updated_config, get value from environment or AWS_DEFAULT_REGION.
        let region = std::env::var("AWS_REGION").unwrap_or_else(|_| {
            std::env::var("AWS_DEFAULT_REGION")
                .unwrap_or_else(|_| AWS_DEFAULT_REGION.to_owned())
        });
        updated_config.insert("AWS_REGION".to_string(), region);
    }
    info!("AWS_REGION used: {:?}", updated_config.get("AWS_REGION"));

    if !config.contains_key("region") {
        updated_config
            .insert("region".to_string(), AWS_DEFAULT_REGION.to_owned());
    }

    if !config.contains_key("AWS_ACCESS_KEY_ID") {
        if let Ok(aws_access_key_id) = env::var("AWS_ACCESS_KEY_ID") {
            updated_config
                .insert("AWS_ACCESS_KEY_ID".to_string(), aws_access_key_id);
        } else {
            return Err("AWS_ACCESS_KEY_ID not found in the config and \
                        environment"
                .into());
        }
    }

    if !config.contains_key("AWS_SECRET_ACCESS_KEY") {
        if let Ok(aws_secret_access_key) = env::var("AWS_SECRET_ACCESS_KEY") {
            updated_config.insert(
                "AWS_SECRET_ACCESS_KEY".to_string(),
                aws_secret_access_key,
            );
        } else {
            return Err("AWS_SECRET_ACCESS_KEY not found in the config and \
                        environment"
                .into());
        }
    }

    Ok(updated_config)
}
