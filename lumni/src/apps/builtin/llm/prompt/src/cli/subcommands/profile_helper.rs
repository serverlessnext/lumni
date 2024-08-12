use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use dirs::home_dir;
use lumni::api::error::ApplicationError;
use serde_json::{json, Map, Value as JsonValue};

use super::{EncryptionHandler, UserProfileDbHandler};
use crate::external as lumni;

pub async fn interactive_profile_creation(
    db_handler: &mut UserProfileDbHandler,
) -> Result<(), ApplicationError> {
    println!("Welcome to the profile creation wizard!");

    let profile_name = get_profile_name()?;
    db_handler.set_profile_name(profile_name.clone());

    let settings = collect_profile_settings()?;

    if ask_for_custom_ssh_key()? {
        setup_custom_encryption(db_handler).await?;
    }
    db_handler
        .create_or_update(&profile_name, &settings)
        .await?;
    println!("Profile '{}' created successfully!", profile_name);

    Ok(())
}

fn get_profile_name() -> Result<String, ApplicationError> {
    print!("Enter a name for the new profile: ");
    io::stdout().flush()?;
    let mut profile_name = String::new();
    io::stdin().read_line(&mut profile_name)?;
    Ok(profile_name.trim().to_string())
}

fn collect_profile_settings() -> Result<JsonValue, ApplicationError> {
    let mut settings = JsonValue::Object(Map::new());

    loop {
        print!("Enter a key (or press Enter to finish): ");
        io::stdout().flush()?;
        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        if key.is_empty() {
            break;
        }

        let value = get_value_for_key(key)?;
        let encrypt = should_encrypt_value()?;

        if encrypt {
            settings[key] = json!({
                "content": value,
                "encryption_key": "",
            });
        } else {
            settings[key] = JsonValue::String(value);
        }
    }

    Ok(settings)
}

fn get_value_for_key(key: &str) -> Result<String, ApplicationError> {
    print!("Enter the value for '{}': ", key);
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

fn should_encrypt_value() -> Result<bool, ApplicationError> {
    print!("Should this value be encrypted? (y/N): ");
    io::stdout().flush()?;
    let mut encrypt = String::new();
    io::stdin().read_line(&mut encrypt)?;
    Ok(encrypt.trim().to_lowercase() == "y")
}

fn ask_for_custom_ssh_key() -> Result<bool, ApplicationError> {
    print!("Do you want to use a custom SSH key for encryption? (y/N): ");
    io::stdout().flush()?;
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    Ok(response.trim().to_lowercase() == "y")
}

async fn setup_custom_encryption(
    db_handler: &mut UserProfileDbHandler,
) -> Result<(), ApplicationError> {
    println!("Setting up custom SSH key for encryption.");

    let default_key_path =
        home_dir().unwrap_or_default().join(".ssh").join("id_rsa");
    let default_key_str = default_key_path.to_str().unwrap_or("~/.ssh/id_rsa");

    loop {
        println!("Default SSH key path: {}", default_key_str);
        print!("Enter the path to your SSH key (or press Enter for default): ");
        io::stdout().flush()?;
        let mut key_path = String::new();
        io::stdin().read_line(&mut key_path)?;
        let key_path = key_path.trim();

        let key_path = if key_path.is_empty() {
            default_key_path.clone()
        } else {
            PathBuf::from(key_path)
        };

        match EncryptionHandler::new_from_path(&key_path) {
            Ok(Some(handler)) => {
                println!("Custom SSH key registered successfully.");
                db_handler.set_encryption_handler(Arc::new(handler));
                break;
            }
            Ok(None) => {
                println!(
                    "Failed to create encryption handler. Please try again \
                     with a valid SSH key path."
                );
            }
            Err(e) => {
                println!("Error registering SSH key: {}. Please try again.", e);
            }
        }
    }

    Ok(())
}
