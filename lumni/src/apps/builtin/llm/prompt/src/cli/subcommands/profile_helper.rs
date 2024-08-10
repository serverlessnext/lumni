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

    // Get profile name
    print!("Enter a name for the new profile: ");
    io::stdout().flush()?;
    let mut profile_name = String::new();
    io::stdin().read_line(&mut profile_name)?;
    let profile_name = profile_name.trim().to_string();

    // Set the profile name in the db_handler
    db_handler.set_profile_name(profile_name.clone());

    // Initialize profile settings
    let mut settings = JsonValue::Object(Map::new());

    // Add key-value pairs
    loop {
        print!("Enter a key (or press Enter to finish): ");
        io::stdout().flush()?;
        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        if key.is_empty() {
            break;
        }

        print!("Enter the value for '{}': ", key);
        io::stdout().flush()?;
        let mut value = String::new();
        io::stdin().read_line(&mut value)?;
        let value = value.trim();

        print!("Should this value be encrypted? (y/N): ");
        io::stdout().flush()?;
        let mut encrypt = String::new();
        io::stdin().read_line(&mut encrypt)?;
        let encrypt = encrypt.trim().to_lowercase() == "y";

        if encrypt {
            settings[key] = json!({
                "content": value,
                "encryption_key": "",
            });
        } else {
            settings[key] = JsonValue::String(value.to_string());
        }
    }

    // Check if any values need encryption
    let needs_encryption = settings
        .as_object()
        .unwrap()
        .values()
        .any(|v| v.is_object() && v.get("encryption_key").is_some());

    // Set up SSH key if needed
    if needs_encryption {
        println!("Some values need encryption. Let's set up an SSH key.");

        let default_key_path =
            home_dir().unwrap_or_default().join(".ssh").join("id_rsa");
        let default_key_str =
            default_key_path.to_str().unwrap_or("~/.ssh/id_rsa");

        loop {
            println!("Default SSH key path: {}", default_key_str);
            print!(
                "Press Enter to use the default, enter a custom path, or type \
                 'exit' to cancel: "
            );
            io::stdout().flush()?;
            let mut key_path = String::new();
            io::stdin().read_line(&mut key_path)?;
            let key_path = key_path.trim();

            if key_path.to_lowercase() == "exit" {
                println!("Profile creation cancelled.");
                return Ok(());
            }

            let key_path = if key_path.is_empty() {
                default_key_path.clone()
            } else {
                PathBuf::from(key_path)
            };

            match EncryptionHandler::new_from_path(Some(&key_path)) {
                Ok(Some(handler)) => {
                    println!("SSH key registered successfully.");
                    // Set the encryption handler in the db_handler
                    db_handler.set_encryption_handler(Arc::new(handler));
                    break;
                }
                Ok(None) => {
                    println!(
                        "Failed to create encryption handler. Please try \
                         again with a valid SSH key path."
                    );
                    continue;
                }
                Err(e) => {
                    println!(
                        "Error registering SSH key: {}. Please try again.",
                        e
                    );
                    continue;
                }
            }
        }
    }

    // Create the profile only if we have a valid encryption handler (if needed)
    if !needs_encryption || db_handler.get_encryption_handler().is_some() {
        db_handler
            .create_or_update(&profile_name, &settings)
            .await?;
        println!("Profile '{}' created successfully!", profile_name);
    } else {
        println!("Profile creation cancelled due to missing encryption key.");
    }

    Ok(())
}
