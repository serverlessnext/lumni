use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};
use lumni::api::error::ApplicationError;
use serde_json::{Map, Value as JsonValue};

use super::profile_helper::interactive_profile_creation;
use super::UserProfileDbHandler;
use crate::external as lumni;

pub fn create_profile_subcommand() -> Command {
    Command::new("profile")
        .about("Manage user profiles")
        .subcommand(create_list_subcommand())
        .subcommand(create_show_subcommand())
        .subcommand(create_set_subcommand())
        .subcommand(create_get_subcommand())
        .subcommand(create_del_subcommand())
        .subcommand(create_rm_subcommand())
        .subcommand(create_set_default_subcommand())
        .subcommand(create_show_default_subcommand())
        .subcommand(create_add_profile_subcommand())
        .subcommand(create_key_subcommand())
}

fn create_list_subcommand() -> Command {
    Command::new("list").about("List all profiles")
}

fn create_show_subcommand() -> Command {
    Command::new("show")
        .about("Show profile settings")
        .arg(Arg::new("name").help("Name of the profile"))
        .arg(
            Arg::new("show-decrypted")
                .long("show-decrypted")
                .help("Show decrypted values instead of masked values")
                .action(ArgAction::SetTrue),
        )
}

fn create_set_subcommand() -> Command {
    Command::new("set")
        .about("Set a profile value")
        .arg(Arg::new("name").help("Name of the profile"))
        .arg(Arg::new("key").help("Key to set"))
        .arg(Arg::new("value").help("Value to set"))
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
        .arg(Arg::new("name").help("Name of the profile"))
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
        .arg(Arg::new("name").help("Name of the profile"))
        .arg(Arg::new("key").help("Key to delete"))
}

fn create_rm_subcommand() -> Command {
    // Renamed from create_delete_subcommand
    Command::new("rm")
        .about("Remove a profile")
        .arg(Arg::new("name").help("Name of the profile to remove"))
}

fn create_set_default_subcommand() -> Command {
    Command::new("set-default")
        .about("Set a profile as default")
        .arg(Arg::new("name").help("Name of the profile"))
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

fn create_add_profile_subcommand() -> Command {
    Command::new("add").about("Add a new profile with guided setup")
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
        .arg(
            Arg::new("type")
                .long("type")
                .help("Type of the key (e.g., 'ssh', 'gpg')")
                .default_value("ssh"),
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

pub async fn handle_profile_subcommand(
    profile_matches: &ArgMatches,
    db_handler: &mut UserProfileDbHandler,
) -> Result<(), ApplicationError> {
    match profile_matches.subcommand() {
        Some(("list", _)) => {
            let profiles = db_handler.list_profiles().await?;
            let default_profile = db_handler.get_default_profile().await?;
            println!("Available profiles:");
            for profile in profiles {
                if Some(&profile) == default_profile.as_ref() {
                    println!("  {} (default)", profile);
                } else {
                    println!("  {}", profile);
                }
            }
        }

        Some(("show", show_matches)) => {
            if show_matches.contains_id("name") {
                eprintln!("Name: {:?}", show_matches.get_one::<String>("name"));
                let profile_name =
                    show_matches.get_one::<String>("name").unwrap();
                let show_decrypted = show_matches.get_flag("show-decrypted");
                let settings = db_handler
                    .get_profile_settings(profile_name, !show_decrypted)
                    .await?;
                println!("Profile '{}' settings:", profile_name);
                for (key, value) in settings.as_object().unwrap() {
                    println!("  {}: {}", key, value);
                }
            } else {
                create_show_subcommand().print_help()?;
            }
        }

        Some(("set", set_matches)) => {
            if set_matches.contains_id("name")
                && set_matches.contains_id("key")
                && set_matches.contains_id("value")
            {
                let profile_name =
                    set_matches.get_one::<String>("name").unwrap();
                let key = set_matches.get_one::<String>("key").unwrap();
                let value = set_matches.get_one::<String>("value").unwrap();
                let is_secure = set_matches.get_flag("secure");

                let mut settings = JsonValue::Object(Map::new());
                if is_secure {
                    settings[key.to_string()] =
                        JsonValue::Object(Map::from_iter(vec![
                            (
                                "content".to_string(),
                                JsonValue::String(value.to_string()),
                            ),
                            (
                                "encryption_key".to_string(),
                                JsonValue::String("".to_string()),
                            ),
                        ]));
                } else {
                    settings[key.to_string()] =
                        JsonValue::String(value.to_string());
                }

                db_handler.create_or_update(profile_name, &settings).await?;
                println!(
                    "Profile '{}' updated. Key '{}' set.",
                    profile_name, key
                );
            } else {
                create_set_subcommand().print_help()?;
            }
        }

        Some(("get", get_matches)) => {
            if get_matches.contains_id("name") && get_matches.contains_id("key")
            {
                let profile_name =
                    get_matches.get_one::<String>("name").unwrap();
                let key = get_matches.get_one::<String>("key").unwrap();
                let show_decrypted = get_matches.get_flag("show-decrypted");
                let settings = db_handler
                    .get_profile_settings(profile_name, !show_decrypted)
                    .await?;
                if let Some(value) = settings.get(key) {
                    println!("{}: {}", key, value);
                } else {
                    println!(
                        "Key '{}' not found in profile '{}'",
                        key, profile_name
                    );
                }
            } else {
                create_get_subcommand().print_help()?;
            }
        }

        Some(("del", del_matches)) => {
            if del_matches.contains_id("name") && del_matches.contains_id("key")
            {
                let profile_name =
                    del_matches.get_one::<String>("name").unwrap();
                let key = del_matches.get_one::<String>("key").unwrap();

                let mut settings = JsonValue::Object(Map::new());
                settings[key.to_string()] = JsonValue::Null; // Null indicates deletion

                db_handler.create_or_update(profile_name, &settings).await?;
                println!(
                    "Key '{}' deleted from profile '{}'.",
                    key, profile_name
                );
            } else {
                create_del_subcommand().print_help()?;
            }
        }

        Some(("rm", rm_matches)) => {
            if rm_matches.contains_id("name") {
                let profile_name =
                    rm_matches.get_one::<String>("name").unwrap();
                db_handler.delete_profile(profile_name).await?;
                println!("Profile '{}' removed.", profile_name);
            } else {
                create_rm_subcommand().print_help()?;
            }
        }

        Some(("set-default", default_matches)) => {
            if default_matches.contains_id("name") {
                let profile_name =
                    default_matches.get_one::<String>("name").unwrap();
                db_handler.set_default_profile(profile_name).await?;
                println!("Profile '{}' set as default.", profile_name);
            } else {
                create_set_default_subcommand().print_help()?;
            }
        }

        Some(("show-default", show_default_matches)) => {
            if let Some(default_profile) =
                db_handler.get_default_profile().await?
            {
                println!("Default profile: {}", default_profile);
                let show_decrypted =
                    show_default_matches.get_flag("show-decrypted");
                let settings = db_handler
                    .get_profile_settings(&default_profile, !show_decrypted)
                    .await?;
                println!("Settings:");
                for (key, value) in settings.as_object().unwrap() {
                    println!("  {}: {}", key, value);
                }
            } else {
                println!("No default profile set.");
            }
        }

        Some(("add", _)) => {
            interactive_profile_creation(db_handler).await?;
        }

        Some(("key", key_matches)) => match key_matches.subcommand() {
            Some(("add", add_matches)) => {
                let name = add_matches.get_one::<String>("name").unwrap();
                let path = add_matches.get_one::<String>("path").unwrap();
                let key_type = add_matches.get_one::<String>("type").unwrap();
                db_handler
                    .register_encryption_key(
                        name,
                        &PathBuf::from(path),
                        key_type,
                    )
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
                let (file_path, sha256_hash, key_type) =
                    db_handler.get_encryption_key(name).await?;
                println!("Encryption key '{}' details:", name);
                println!("  File path: {}", file_path);
                println!("  SHA256 hash: {}", sha256_hash);
                println!("  Key type: {}", key_type);
            }
            _ => {
                create_key_subcommand().print_help()?;
            }
        },

        _ => {
            create_profile_subcommand().print_help()?;
        }
    }

    Ok(())
}
