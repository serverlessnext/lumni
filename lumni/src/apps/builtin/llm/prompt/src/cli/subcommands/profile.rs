use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};
use lumni::api::error::ApplicationError;
use serde_json::{json, Map, Number as JsonNumber, Value as JsonValue};

use super::{MaskMode, UserProfile, UserProfileDbHandler};
use crate::external as lumni;

pub fn create_profile_subcommand() -> Command {
    Command::new("profile")
        .about("Manage user profiles")
        .subcommand(create_list_subcommand())
        .subcommand(create_show_subcommand())
        .subcommand(create_create_subcommand())
        .subcommand(create_set_subcommand())
        .subcommand(create_get_subcommand())
        .subcommand(create_del_subcommand())
        .subcommand(create_rm_subcommand())
        .subcommand(create_set_default_subcommand())
        .subcommand(create_show_default_subcommand())
        .subcommand(create_key_subcommand())
        .subcommand(create_export_subcommand())
        .subcommand(create_truncate_subcommand())
}

fn create_list_subcommand() -> Command {
    Command::new("list").about("List all profiles")
}

fn create_show_subcommand() -> Command {
    Command::new("show")
        .about("Show profile settings")
        .arg(Arg::new("id").help("ID of the profile"))
        .arg(
            Arg::new("show-decrypted")
                .long("show-decrypted")
                .help("Show decrypted values instead of masked values")
                .action(ArgAction::SetTrue),
        )
}

fn create_create_subcommand() -> Command {
    Command::new("create")
        .about("Create a new profile")
        .arg(
            Arg::new("name")
                .help("Name of the new profile")
                .required(true),
        )
        .arg(
            Arg::new("settings")
                .long("settings")
                .help("Initial settings for the profile (JSON string)")
                .required(false),
        )
}

fn create_set_subcommand() -> Command {
    Command::new("set")
        .about("Set a profile value")
        .arg(Arg::new("id").help("ID of the profile"))
        .arg(Arg::new("key").help("Key to set"))
        .arg(Arg::new("value").help("Value to set"))
        .arg(
            Arg::new("type")
                .long("type")
                .help(
                    "Specify the type of the value (string, number, boolean, \
                     null, array, object)",
                )
                .value_parser([
                    "string", "number", "boolean", "null", "array", "object",
                ])
                .default_value("string"),
        )
        .arg(
            Arg::new("secure")
                .long("secure")
                .help("Mark the value as secure (to be encrypted)")
                .action(ArgAction::SetTrue),
        )
}

fn create_get_subcommand() -> Command {
    Command::new("get")
        .about("Get a specific profile value")
        .arg(Arg::new("id").help("ID of the profile"))
        .arg(Arg::new("key").help("Key to get"))
        .arg(
            Arg::new("show-decrypted")
                .long("show-decrypted")
                .help("Show decrypted value instead of masked value")
                .action(ArgAction::SetTrue),
        )
}

fn create_del_subcommand() -> Command {
    Command::new("del")
        .about("Delete a key from a profile")
        .arg(Arg::new("id").help("ID of the profile"))
        .arg(Arg::new("key").help("Key to delete"))
}

fn create_rm_subcommand() -> Command {
    Command::new("rm")
        .about("Remove a profile")
        .arg(Arg::new("id").help("ID of the profile to remove"))
}

fn create_set_default_subcommand() -> Command {
    Command::new("set-default")
        .about("Set a profile as default")
        .arg(Arg::new("id").help("ID of the profile"))
}

fn create_show_default_subcommand() -> Command {
    Command::new("show-default")
        .about("Show the default profile")
        .arg(
            Arg::new("show-decrypted")
                .long("show-decrypted")
                .help("Show decrypted values instead of masked values")
                .action(ArgAction::SetTrue),
        )
}

fn create_key_subcommand() -> Command {
    Command::new("key")
        .about("Manage encryption keys for profiles")
        .subcommand(create_key_add_subcommand())
        .subcommand(create_key_list_subcommand())
        .subcommand(create_key_remove_subcommand())
        .subcommand(create_key_show_subcommand())
}

fn create_key_add_subcommand() -> Command {
    Command::new("add")
        .about("Add a new encryption key")
        .arg(Arg::new("name").help("Name for the key").required(true))
        .arg(
            Arg::new("path")
                .help("Path to the private key file")
                .required(true),
        )
}

fn create_key_list_subcommand() -> Command {
    Command::new("list")
        .about("List all registered encryption keys")
        .arg(Arg::new("type").long("type").help("Filter keys by type"))
}

fn create_key_remove_subcommand() -> Command {
    Command::new("remove")
        .about("Remove a registered encryption key")
        .arg(
            Arg::new("name")
                .help("Name of the key to remove")
                .required(true),
        )
}

fn create_key_show_subcommand() -> Command {
    Command::new("show")
        .about("Show details of a specific encryption key")
        .arg(
            Arg::new("name")
                .help("Name of the key to show")
                .required(true),
        )
}

fn create_export_subcommand() -> Command {
    Command::new("export")
        .about("Export profile(s) to JSON")
        .arg(
            Arg::new("id")
                .help(
                    "ID of the profile to export (omit to export all profiles)",
                )
                .required(false),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file path (if not specified, prints to stdout)")
                .value_name("FILE"),
        )
}

fn create_truncate_subcommand() -> Command {
    Command::new("truncate")
        .about("Remove all profiles and related data")
        .arg(
            Arg::new("confirm")
                .short('y')
                .long("yes")
                .help("Confirm the truncation without prompting")
                .action(ArgAction::SetTrue),
        )
}

pub async fn handle_profile_subcommand(
    profile_matches: &ArgMatches,
    mut db_handler: UserProfileDbHandler,
) -> Result<(), ApplicationError> {
    match profile_matches.subcommand() {
        Some(("list", _)) => {
            let profiles = db_handler.list_profiles().await?;
            let default_profile = db_handler.get_default_profile().await?;
            println!("Available profiles:");
            for profile in profiles {
                if Some(&profile) == default_profile.as_ref() {
                    println!(
                        "  ID: {} - {} (default)",
                        profile.id, profile.name
                    );
                } else {
                    println!("  ID: {} - {}", profile.id, profile.name);
                }
            }
        }

        Some(("show", show_matches)) => {
            if let Some(id_str) = show_matches.get_one::<String>("id") {
                let mask_mode = if show_matches.get_flag("show-decrypted") {
                    MaskMode::Unmask
                } else {
                    MaskMode::Mask
                };
                let profile = get_profile_by_id(&db_handler, id_str).await?;
                let settings = db_handler
                    .get_profile_settings(&profile, mask_mode)
                    .await?;
                println!(
                    "Profile ID: {} - {} settings:",
                    profile.id, profile.name
                );
                for (key, value) in settings.as_object().unwrap() {
                    println!("  {}: {}", key, extract_value(value));
                }
            } else {
                create_show_subcommand().print_help()?;
            }
        }

        Some(("create", create_matches)) => {
            let name = create_matches.get_one::<String>("name").unwrap();
            let settings = if let Some(settings_str) =
                create_matches.get_one::<String>("settings")
            {
                serde_json::from_str(settings_str).map_err(|e| {
                    ApplicationError::InvalidInput(format!(
                        "Invalid JSON for settings: {}",
                        e
                    ))
                })?
            } else {
                JsonValue::Object(Map::new())
            };

            let new_profile =
                db_handler.create_profile(name.clone(), settings).await?;
            println!(
                "Created new profile - ID: {}, Name: {}",
                new_profile.id, new_profile.name
            );
        }

        Some(("set", set_matches)) => {
            if let (Some(id_str), Some(key), Some(value), Some(type_str)) = (
                set_matches.get_one::<String>("id"),
                set_matches.get_one::<String>("key"),
                set_matches.get_one::<String>("value"),
                set_matches.get_one::<String>("type"),
            ) {
                let is_secure = set_matches.get_flag("secure");
                let profile = get_profile_by_id(&db_handler, id_str).await?;

                let typed_value = parse_and_validate_value(value, type_str)?;

                let mut settings = JsonValue::Object(serde_json::Map::new());
                if is_secure {
                    settings[key.to_string()] =
                        JsonValue::Object(serde_json::Map::from_iter(vec![
                            ("content".to_string(), typed_value),
                            (
                                "encryption_key".to_string(),
                                JsonValue::String("".to_string()),
                            ),
                        ]));
                } else {
                    settings[key.to_string()] = typed_value;
                }

                db_handler
                    .update_configuration_item(
                        &profile.clone().into(),
                        &settings,
                    )
                    .await?;
                println!(
                    "Profile ID: {} - {} updated. Key '{}' set.",
                    profile.id, profile.name, key
                );
            } else {
                create_set_subcommand().print_help()?;
            }
        }

        Some(("get", get_matches)) => {
            if let (Some(id_str), Some(key)) = (
                get_matches.get_one::<String>("id"),
                get_matches.get_one::<String>("key"),
            ) {
                let mask_mode = if get_matches.get_flag("show-decrypted") {
                    MaskMode::Unmask
                } else {
                    MaskMode::Mask
                };
                let profile = get_profile_by_id(&db_handler, id_str).await?;
                let settings = db_handler
                    .get_profile_settings(&profile, mask_mode)
                    .await?;
                if let Some(value) = settings.get(key) {
                    println!("{}: {}", key, extract_value(value));
                } else {
                    println!(
                        "Key '{}' not found in profile ID: {} - {}",
                        key, profile.id, profile.name
                    );
                }
            } else {
                create_get_subcommand().print_help()?;
            }
        }

        Some(("del", del_matches)) => {
            if let (Some(id_str), Some(key)) = (
                del_matches.get_one::<String>("id"),
                del_matches.get_one::<String>("key"),
            ) {
                let profile = get_profile_by_id(&db_handler, id_str).await?;
                let mut settings = JsonValue::Object(Map::new());
                settings[key.to_string()] = JsonValue::Null; // Null indicates deletion
                db_handler
                    .update_configuration_item(
                        &profile.clone().into(),
                        &settings,
                    )
                    .await?;
                println!(
                    "Key '{}' deleted from profile ID: {} - {}.",
                    key, profile.id, profile.name
                );
            } else {
                create_del_subcommand().print_help()?;
            }
        }

        Some(("rm", rm_matches)) => {
            if let Some(id_str) = rm_matches.get_one::<String>("id") {
                let profile = get_profile_by_id(&db_handler, id_str).await?;
                db_handler.delete_profile(&profile).await?;
                println!(
                    "Profile ID: {} - {} removed.",
                    profile.id, profile.name
                );
            } else {
                create_rm_subcommand().print_help()?;
            }
        }

        Some(("set-default", default_matches)) => {
            if let Some(id_str) = default_matches.get_one::<String>("id") {
                let profile = get_profile_by_id(&db_handler, id_str).await?;
                db_handler.set_default_profile(&profile).await?;
                println!(
                    "Profile ID: {} - {} set as default.",
                    profile.id, profile.name
                );
            } else {
                create_set_default_subcommand().print_help()?;
            }
        }

        Some(("show-default", show_default_matches)) => {
            if let Some(default_profile) =
                db_handler.get_default_profile().await?
            {
                println!(
                    "Default profile ID: {} - {}",
                    default_profile.id, default_profile.name
                );
                let mask_mode =
                    if show_default_matches.get_flag("show-decrypted") {
                        MaskMode::Unmask
                    } else {
                        MaskMode::Mask
                    };
                let settings = db_handler
                    .get_profile_settings(&default_profile, mask_mode)
                    .await?;
                println!("Settings:");
                for (key, value) in settings.as_object().unwrap() {
                    println!("  {}: {}", key, extract_value(value));
                }
            } else {
                println!("No default profile set.");
            }
        }

        Some(("export", export_matches)) => {
            let output_file = export_matches.get_one::<String>("output");
            let default_profile = db_handler.get_default_profile().await?;

            let profiles = if let Some(id_str) =
                export_matches.get_one::<String>("id")
            {
                // Export a single profile
                let profile = get_profile_by_id(&db_handler, id_str).await?;
                let settings =
                    db_handler.export_profile_settings(&profile).await?;
                vec![json!({
                    "ID": profile.id,
                    "Name": profile.name,
                    "Parameters": settings["Parameters"],
                    "EncryptionKey": settings["EncryptionKey"]
                })]
            } else {
                // Export all profiles
                let mut profiles_vec = Vec::new();
                let profile_list = db_handler.list_profiles().await?;
                for profile in profile_list {
                    let settings =
                        db_handler.export_profile_settings(&profile).await?;
                    profiles_vec.push(json!({
                        "ID": profile.id,
                        "Name": profile.name,
                        "Parameters": settings["Parameters"],
                        "EncryptionKey": settings["EncryptionKey"]
                    }));
                }
                profiles_vec
            };

            let mut export_data = json!({
                "Profiles": profiles,
            });

            // Add DefaultProfile field only if it's set
            if let Some(default) = default_profile {
                export_data["DefaultProfile"] = json!({
                    "ID": default.id,
                    "Name": default.name
                });
            }

            export_json(
                &export_data,
                output_file,
                "Profiles exported to JSON",
            )?;
        }

        Some(("key", key_matches)) => match key_matches.subcommand() {
            Some(("add", add_matches)) => {
                let name = add_matches.get_one::<String>("name").unwrap();
                let path = add_matches.get_one::<String>("path").unwrap();
                db_handler
                    .register_encryption_key(name, &PathBuf::from(path))
                    .await?;
                println!("Encryption key '{}' added successfully.", name);
            }
            Some(("list", list_matches)) => {
                let key_type = list_matches.get_one::<String>("type");
                let key_type_str = key_type.map(|s| s.as_str());
                let keys =
                    db_handler.list_encryption_keys(key_type_str).await?;
                println!("Registered encryption keys:");
                for key in keys {
                    println!("  {}", key);
                }
            }
            Some(("remove", remove_matches)) => {
                let name = remove_matches.get_one::<String>("name").unwrap();
                db_handler.remove_encryption_key(name).await?;
                println!("Encryption key '{}' removed successfully.", name);
            }
            Some(("show", show_matches)) => {
                let name = show_matches.get_one::<String>("name").unwrap();
                let (file_path, sha256_hash) =
                    db_handler.get_encryption_key(name).await?;
                println!("Encryption key '{}' details:", name);
                println!("  File path: {}", file_path);
                println!("  SHA256 hash: {}", sha256_hash);
            }
            _ => {
                create_key_subcommand().print_help()?;
            }
        },

        Some(("truncate", truncate_matches)) => {
            if truncate_matches.get_flag("confirm") {
                db_handler.truncate_and_vacuum().await?;
                println!("All profiles and related data have been removed.");
            } else {
                println!(
                    "Are you sure you want to remove all profiles and related \
                     data? This action cannot be undone."
                );
                println!("Type 'yes' to confirm or any other input to cancel:");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().eq_ignore_ascii_case("yes") {
                    db_handler.truncate_and_vacuum().await?;
                    println!(
                        "All profiles and related data have been removed."
                    );
                } else {
                    println!("Operation cancelled.");
                }
            }
        }

        _ => {
            create_profile_subcommand().print_help()?;
        }
    }

    Ok(())
}

fn export_json(
    json: &JsonValue,
    output_file: Option<&String>,
    success_message: &str,
) -> Result<(), ApplicationError> {
    let json_string = serde_json::to_string_pretty(json)?;
    if let Some(file_path) = output_file {
        std::fs::write(file_path, json_string)?;
        println!("{}. Saved to: {}", success_message, file_path);
    } else {
        println!("{}", json_string);
    }

    Ok(())
}

fn extract_value(value: &JsonValue) -> &JsonValue {
    if let Some(obj) = value.as_object() {
        if obj.contains_key("was_encrypted") {
            obj.get("content").unwrap_or(value)
        } else {
            value
        }
    } else {
        value
    }
}

async fn get_profile_by_id(
    db_handler: &UserProfileDbHandler,
    id_str: &str,
) -> Result<UserProfile, ApplicationError> {
    let id = id_str.parse::<i64>().map_err(|_| {
        ApplicationError::InvalidInput(format!(
            "Invalid profile ID: {}",
            id_str
        ))
    })?;

    match db_handler.get_profile_by_id(id).await? {
        Some(profile) => Ok(profile),
        None => Err(ApplicationError::InvalidInput(format!(
            "No profile found with ID: {}",
            id
        ))),
    }
}

fn parse_and_validate_value(
    value: &str,
    type_str: &str,
) -> Result<JsonValue, ApplicationError> {
    match type_str {
        "string" => Ok(JsonValue::String(value.to_string())),
        "number" => {
            // First, try parsing as an integer
            if let Ok(int_value) = value.parse::<i64>() {
                Ok(JsonValue::Number(int_value.into()))
            } else {
                // If not an integer, try parsing as a float
                value
                    .parse::<f64>()
                    .map_err(|_| {
                        ApplicationError::InvalidInput(format!(
                            "Invalid number: {}",
                            value
                        ))
                    })
                    .and_then(|float_value| {
                        JsonNumber::from_f64(float_value)
                            .map(JsonValue::Number)
                            .ok_or_else(|| {
                                ApplicationError::InvalidInput(format!(
                                    "Invalid number: {}",
                                    value
                                ))
                            })
                    })
            }
        }
        "boolean" => match value.to_lowercase().as_str() {
            "true" => Ok(JsonValue::Bool(true)),
            "false" => Ok(JsonValue::Bool(false)),
            _ => Err(ApplicationError::InvalidInput(format!(
                "Invalid boolean: {}",
                value
            ))),
        },
        "null" => {
            if value.to_lowercase() == "null" {
                Ok(JsonValue::Null)
            } else {
                Err(ApplicationError::InvalidInput(format!(
                    "Invalid null value: {}",
                    value
                )))
            }
        }
        "array" => serde_json::from_str(value).map_err(|_| {
            ApplicationError::InvalidInput(format!(
                "Invalid JSON array: {}",
                value
            ))
        }),
        "object" => serde_json::from_str(value).map_err(|_| {
            ApplicationError::InvalidInput(format!(
                "Invalid JSON object: {}",
                value
            ))
        }),
        _ => Err(ApplicationError::InvalidInput(format!(
            "Invalid type: {}",
            type_str
        ))),
    }
}
