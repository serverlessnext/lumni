use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use lumni::api::error::ApplicationError;
use serde_json::Value as JsonValue;

use super::UserProfileDbHandler;
use crate::external as lumni;

pub fn create_profile_subcommand() -> Command {
    Command::new("profile")
        .about("Manage user profiles")
        .arg(Arg::new("name").help("Name of the profile").index(1))
        .arg(
            Arg::new("set")
                .long("set")
                .short('s')
                .help("Set profile values")
                .num_args(2)
                .value_names(["KEY", "VALUE"])
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("get")
                .long("get")
                .short('g')
                .help("Get a specific profile value")
                .num_args(1)
                .value_name("KEY"),
        )
        .arg(
            Arg::new("show")
                .long("show")
                .help("Show all profile values")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("delete")
                .long("delete")
                .short('d')
                .help("Delete the profile")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("default")
                .long("default")
                .help("Set as the default profile")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List all profiles")
                .action(ArgAction::SetTrue),
        )
        .group(
            ArgGroup::new("profile_group")
                .args(["set", "get", "show", "delete", "default", "list"])
                .required(false)
                .multiple(false),
        )
}

pub async fn handle_profile_subcommand(
    profile_matches: &ArgMatches,
    db_handler: &mut UserProfileDbHandler,
) -> Result<(), ApplicationError> {
    if profile_matches.get_flag("list") {
        let profiles = db_handler.list_profiles().await?;
        println!("Available profiles:");
        for profile in profiles {
            println!("  {}", profile);
        }
        return Ok(());
    }

    let profile_name = profile_matches.get_one::<String>("name");

    if profile_matches.contains_id("set") {
        if let Some(profile_name) = profile_name {
            let mut settings = JsonValue::default();
            let values: Vec<&str> = profile_matches
                .get_many::<String>("set")
                .unwrap()
                .map(AsRef::as_ref)
                .collect();
            for chunk in values.chunks(2) {
                if let [key, value] = chunk {
                    settings[key.to_string()] =
                        JsonValue::String(value.to_string());
                }
            }
            db_handler.create_or_update(profile_name, &settings).await?;
            println!("Profile '{}' updated.", profile_name);
        } else {
            println!("Error: Profile name is required for set operation");
        }
    } else if let Some(key) = profile_matches.get_one::<String>("get") {
        if let Some(profile_name) = profile_name {
            let settings =
                db_handler.get_profile_settings(profile_name).await?;
            if let Some(value) = settings.get(key) {
                println!("{}: {}", key, value);
            } else {
                println!(
                    "Key '{}' not found in profile '{}'",
                    key, profile_name
                );
            }
        } else {
            println!("Error: Profile name is required for get operation");
        }
    } else if profile_matches.get_flag("show") {
        if let Some(profile_name) = profile_name {
            let settings =
                db_handler.get_profile_settings(profile_name).await?;
            println!("Profile '{}' settings:", profile_name);
            for (key, value) in settings.as_object().unwrap() {
                println!("  {}: {}", key, value);
            }
        } else {
            println!("Error: Profile name is required for show operation");
        }
    } else if profile_matches.get_flag("delete") {
        if let Some(profile_name) = profile_name {
            db_handler.delete_profile(profile_name).await?;
            println!("Profile '{}' deleted.", profile_name);
        } else {
            println!("Error: Profile name is required for delete operation");
        }
    } else if profile_matches.get_flag("default") {
        if let Some(profile_name) = profile_name {
            db_handler.set_default_profile(profile_name).await?;
            println!("Profile '{}' set as default.", profile_name);
        } else {
            println!("Error: Profile name is required to set as default");
        }
    } else if profile_name.is_some() {
        // If a profile name is provided but no action is specified, show that profile
        let profile_name = profile_name.unwrap();
        let settings = db_handler.get_profile_settings(profile_name).await?;
        println!("Profile '{}' settings:", profile_name);
        for (key, value) in settings.as_object().unwrap() {
            println!("  {}: {}", key, value);
        }
    } else {
        let mut profile_command = create_profile_subcommand();
        profile_command.print_help()?;
    }

    Ok(())
}
