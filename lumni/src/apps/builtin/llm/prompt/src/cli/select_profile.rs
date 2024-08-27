use std::str::FromStr;

use lumni::api::error::ApplicationError;

use super::UserProfileDbHandler;
use crate::external as lumni;

pub async fn handle_profile_selection(
    profile_handler: &mut UserProfileDbHandler,
    profile_arg: Option<&String>,
) -> Result<(), ApplicationError> {
    if let Some(profile_selector) = profile_arg {
        let (name, id) = parse_profile_selector(profile_selector);

        match (name, id) {
            (Some(name), Some(id)) => {
                // Case: name::id
                select_profile_by_name_and_id(profile_handler, name, id)
                    .await?;
            }
            (Some(name), None) => {
                // Case: name
                select_profile_by_name(profile_handler, name).await?;
            }
            (None, Some(id)) => {
                // Case: ::id
                select_profile_by_id(profile_handler, id).await?;
            }
            _ => {
                return Err(ApplicationError::InvalidInput(
                    "Invalid profile selector format".to_string(),
                ));
            }
        }
    } else {
        // Use default profile if set
        if let Some(default_profile) =
            profile_handler.get_default_profile().await?
        {
            profile_handler.set_profile(default_profile);
        } else {
            return Err(ApplicationError::InvalidInput(
                "No profile set and no default profile available".to_string(),
            ));
        }
    }

    // Check if a profile is set
    if profile_handler.get_profile().is_none() {
        return Err(ApplicationError::InvalidInput(
            "No profile set".to_string(),
        ));
    }

    Ok(())
}

fn parse_profile_selector(selector: &str) -> (Option<&str>, Option<i64>) {
    if selector.starts_with("::") {
        // Case: ::id
        let id_str = selector.trim_start_matches("::");
        return (None, i64::from_str(id_str).ok());
    }

    let parts: Vec<&str> = selector.split("::").collect();
    match parts.as_slice() {
        [name, id] => (Some(name.trim()), i64::from_str(id.trim()).ok()),
        [name] => (Some(name.trim()), None),
        _ => (None, None),
    }
}

async fn select_profile_by_name_and_id(
    profile_handler: &mut UserProfileDbHandler,
    name: &str,
    id: i64,
) -> Result<(), ApplicationError> {
    if let Some(profile) = profile_handler.get_profile_by_id(id).await? {
        if profile.name == name {
            profile_handler.set_profile(profile);
            Ok(())
        } else {
            Err(ApplicationError::InvalidInput(format!(
                "Profile with id {} does not match the name '{}'",
                id, name
            )))
        }
    } else {
        Err(ApplicationError::InvalidInput(format!(
            "No profile found with id {}",
            id
        )))
    }
}

async fn select_profile_by_name(
    profile_handler: &mut UserProfileDbHandler,
    name: &str,
) -> Result<(), ApplicationError> {
    let profiles = profile_handler.get_profiles_by_name(name).await?;
    match profiles.len() {
        0 => Err(ApplicationError::InvalidInput(format!(
            "No profile found with name '{}'",
            name
        ))),
        1 => {
            profile_handler.set_profile(profiles[0].clone());
            Ok(())
        }
        _ => {
            println!(
                "Multiple profiles found with the name '{}'. Please specify \
                 the id:",
                name
            );
            for profile in profiles {
                println!("  ID: {}, Name: {}", profile.id, profile.name);
            }
            Err(ApplicationError::InvalidInput(
                "Multiple profiles found. Please specify the id.".to_string(),
            ))
        }
    }
}

async fn select_profile_by_id(
    profile_handler: &mut UserProfileDbHandler,
    id: i64,
) -> Result<(), ApplicationError> {
    if let Some(profile) = profile_handler.get_profile_by_id(id).await? {
        profile_handler.set_profile(profile);
        Ok(())
    } else {
        Err(ApplicationError::InvalidInput(format!(
            "No profile found with id {}",
            id
        )))
    }
}
