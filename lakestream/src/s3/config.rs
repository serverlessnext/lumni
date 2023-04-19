use std::collections::HashMap;
use std::env;

use crate::AWS_DEFAULT_REGION;

pub fn update_config(
    config: &HashMap<String, String>,
) -> Result<HashMap<String, String>, &'static str> {
    let mut updated_config = config.clone();

    if !config.contains_key("region") {
        updated_config
            .insert("region".to_string(), AWS_DEFAULT_REGION.to_owned());
    }

    if !config.contains_key("access_key") {
        if let Ok(aws_access_key_id) = env::var("AWS_ACCESS_KEY_ID") {
            updated_config.insert("access_key".to_string(), aws_access_key_id);
        } else {
            return Err(
                "AWS_ACCESS_KEY_ID not found in the config and environment"
            );
        }
    }

    if !config.contains_key("secret_key") {
        if let Ok(aws_secret_access_key) = env::var("AWS_SECRET_ACCESS_KEY") {
            updated_config
                .insert("secret_key".to_string(), aws_secret_access_key);
        } else {
            return Err(
                "AWS_SECRET_ACCESS_KEY not found in the config and environment"
            );
        }
    }

    Ok(updated_config)
}
