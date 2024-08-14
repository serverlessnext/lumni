use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use lumni::api::error::ApplicationError;
use serde_json::{json, Map, Value as JsonValue};

use super::{
    EncryptionHandler, MaskMode, ModelServer, ModelSpec, ServerTrait,
    UserProfileDbHandler, SUPPORTED_MODEL_ENDPOINTS,
};
use crate::external as lumni;

enum ModelSelection {
    Selected(ModelSpec),
    Reload,
    Quit,
    Skip,
}

pub async fn interactive_profile_edit(
    db_handler: &mut UserProfileDbHandler,
    profile_name_to_update: Option<String>,
    custom_ssh_key_path: Option<String>,
) -> Result<(), ApplicationError> {
    println!("Welcome to the profile creation/editing wizard!");

    let profile_name = match &profile_name_to_update {
        Some(name) => name.clone(),
        None => get_profile_name()?,
    };
    db_handler.set_profile_name(profile_name.clone());

    // validate key first if provided
    if custom_ssh_key_path.is_some() {
        setup_custom_encryption(db_handler, custom_ssh_key_path.as_ref().unwrap()).await?;
    }

    let (mut settings, is_updating) = match profile_name_to_update {
        Some(name) => match db_handler
            .get_profile_settings(&name, MaskMode::Unmask)
            .await
        {
            Ok(existing_settings) => (existing_settings, true),
            Err(ApplicationError::DatabaseError(_)) => {
                println!(
                    "Profile '{}' does not exist. Creating a new profile.",
                    name
                );
                (JsonValue::Object(Map::new()), false)
            }
            Err(e) => return Err(e),
        },
        None => (JsonValue::Object(Map::new()), false),
    };

    let profile_type = if is_updating {
        settings["__PROFILE_TYPE"]
            .as_str()
            .unwrap_or("Custom")
            .to_string()
    } else {
        let selected_type = select_profile_type()?;
        settings["__PROFILE_TYPE"] = JsonValue::String(selected_type.clone());
        selected_type
    };

    if !is_updating && profile_type != "Custom" {
        let model_server = ModelServer::from_str(&profile_type)?;

        if let Some(selected_model) = select_model(&model_server).await? {
            settings["__MODEL_IDENTIFIER"] =
                JsonValue::String(selected_model.identifier.0);
        } else {
            println!("No model selected. Skipping model selection.");
        }

        // Get other predefined settings
        let server_settings = model_server.get_profile_settings();
        if let JsonValue::Object(map) = server_settings {
            for (key, value) in map {
                if settings.get(&key).is_none() {
                    settings[key] = value;
                }
            }
        }
    }

    if !is_updating || settings.get("__PROJECT_DIRECTORY").is_none() {
        if ask_yes_no("Do you want to set a project directory?", true)? {
            let dir = get_project_directory()?;
            settings["__PROJECT_DIRECTORY"] = JsonValue::String(dir);
        }
    }

    collect_profile_settings(&mut settings, is_updating)?;

    // Allow adding custom keys, but default to No if updating or if a specific profile type is chosen
    if ask_yes_no(
        "Do you want to add custom keys?",
        !is_updating && profile_type == "Custom",
    )? {
        collect_custom_settings(&mut settings)?;
    }


    db_handler
        .create_or_update(&profile_name, &settings)
        .await?;

    println!(
        "Profile '{}' {} successfully!",
        profile_name,
        if is_updating { "updated" } else { "created" }
    );

    Ok(())
}

fn collect_profile_settings(
    settings: &mut JsonValue,
    is_updating: bool,
) -> Result<(), ApplicationError> {
    if let JsonValue::Object(ref mut map) = settings {
        for (key, value) in map.clone().iter() {
            if key.starts_with("__") {
                // Skip protected values when editing
                if is_updating {
                    continue;
                }
                // For new profiles, just display the value of protected settings
                println!("{}: {}", key, value);
                continue;
            }

            let current_value = parse_value(value);

            let prompt = if is_updating {
                format!(
                    "Current value for '{}' is '{}'. Enter new value (or \
                     press Enter to keep current): ",
                    key, current_value
                )
            } else {
                format!("Enter value for '{}': ", key)
            };

            print!("{}", prompt);
            io::stdout().flush()?;
            let mut new_value = String::new();
            io::stdin().read_line(&mut new_value)?;
            let new_value = new_value.trim();

            if !new_value.is_empty() {
                match value {
                    JsonValue::Object(obj)
                        if obj.contains_key("content")
                            && obj.contains_key("encryption_key") =>
                    {
                        // This is a predefined encrypted value, maintain its structure
                        map.insert(
                            key.to_string(),
                            json!({
                                "content": new_value,
                                "encryption_key": "",
                            }),
                        );
                    }
                    JsonValue::Null => {
                        // This is a predefined non-encrypted value
                        map.insert(
                            key.to_string(),
                            JsonValue::String(new_value.to_string()),
                        );
                    }
                    _ => {
                        // For custom keys or updating existing values
                        if !is_updating && should_encrypt_value()? {
                            map.insert(
                                key.to_string(),
                                json!({
                                    "content": new_value,
                                    "encryption_key": "",
                                }),
                            );
                        } else {
                            map.insert(
                                key.to_string(),
                                JsonValue::String(new_value.to_string()),
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn collect_custom_settings(
    settings: &mut JsonValue,
) -> Result<(), ApplicationError> {
    loop {
        print!("Enter a new custom key (or press Enter to finish): ");
        io::stdout().flush()?;
        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        if key.is_empty() {
            break;
        }

        if key.starts_with("__") {
            println!(
                "Keys starting with '__' are reserved. Please choose a \
                 different key."
            );
            continue;
        }

        if let JsonValue::Object(ref map) = settings {
            if map.get(key).is_some() {
                println!(
                    "Key '{}' already exists. Please choose a different key.",
                    key
                );
                continue;
            }
        }

        let value = get_value_for_key(key)?;
        let encrypt = should_encrypt_value()?;

        if let JsonValue::Object(ref mut map) = settings {
            if encrypt {
                map.insert(
                    key.to_string(),
                    json!({
                        "content": value,
                        "encryption_key": "",
                    }),
                );
            } else {
                map.insert(key.to_string(), JsonValue::String(value));
            }
        }
    }

    Ok(())
}

fn parse_value(value: &JsonValue) -> String {
    match value {
        JsonValue::Object(obj)
            if obj.contains_key("content")
                && obj.contains_key("encryption_key") =>
        {
            obj.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        }
        JsonValue::Null => "[Not set]".to_string(),
        _ => value.as_str().unwrap_or_default().to_string(),
    }
}

async fn select_model(
    model_server: &ModelServer,
) -> Result<Option<ModelSpec>, ApplicationError> {
    loop {
        match model_server.list_models().await {
            Ok(models) => {
                if models.is_empty() {
                    println!("No models available for this server.");
                    return Ok(None);
                }

                match select_model_from_list(&models)? {
                    ModelSelection::Selected(model) => return Ok(Some(model)),
                    ModelSelection::Reload => {
                        println!("Reloading model list...");
                        continue;
                    }
                    ModelSelection::Quit => {
                        return Err(ApplicationError::UserCancelled(
                            "Model selection cancelled by user".to_string(),
                        ))
                    }
                    ModelSelection::Skip => return Ok(None),
                }
            }
            Err(ApplicationError::NotReady(msg)) => {
                println!("Error: {}", msg);
                match handle_not_ready()? {
                    ModelSelection::Reload => continue,
                    ModelSelection::Quit => {
                        return Err(ApplicationError::UserCancelled(
                            "Model selection cancelled by user".to_string(),
                        ))
                    }
                    ModelSelection::Skip => return Ok(None),
                    _ => unreachable!(),
                }
            }
            Err(e) => return Err(e), // propagate other errors
        }
    }
}

fn select_model_from_list(
    models: &[ModelSpec],
) -> Result<ModelSelection, ApplicationError> {
    println!("Available models:");
    for (index, model) in models.iter().enumerate() {
        println!("{}. {}", index + 1, model.identifier.0);
    }

    print!(
        "Select a model (1-{}), press Enter to reload, 'q' to quit, or 's' to \
         skip: ",
        models.len()
    );
    io::stdout().flush()?;
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let choice = choice.trim().to_lowercase();

    match choice.as_str() {
        "" => Ok(ModelSelection::Reload),
        "q" => {
            println!("Quitting model selection.");
            Ok(ModelSelection::Quit)
        }
        "s" => {
            println!("Skipping model selection.");
            Ok(ModelSelection::Skip)
        }
        _ => {
            if let Ok(index) = choice.parse::<usize>() {
                if index > 0 && index <= models.len() {
                    return Ok(ModelSelection::Selected(
                        models[index - 1].clone(),
                    ));
                }
            }
            println!("Invalid choice. Please try again.");
            select_model_from_list(models) // Recursively ask for selection again
        }
    }
}

fn handle_not_ready() -> Result<ModelSelection, ApplicationError> {
    loop {
        print!(
            "Press Enter to retry, 'q' to quit, or 's' to skip model \
             selection: "
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "" => return Ok(ModelSelection::Reload),
            "q" => return Ok(ModelSelection::Quit),
            "s" => return Ok(ModelSelection::Skip),
            _ => println!("Invalid input. Please try again."),
        }
    }
}

fn select_profile_type() -> Result<String, ApplicationError> {
    println!("Select a profile type:");
    println!("0. Custom (default)");
    for (index, server_type) in SUPPORTED_MODEL_ENDPOINTS.iter().enumerate() {
        println!("{}. {}", index + 1, server_type);
    }

    loop {
        print!(
            "Enter your choice (0-{}, or press Enter for Custom): ",
            SUPPORTED_MODEL_ENDPOINTS.len()
        );
        io::stdout().flush()?;
        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        if choice.is_empty() {
            return Ok("Custom".to_string());
        }

        if let Ok(index) = choice.parse::<usize>() {
            if index == 0 {
                return Ok("Custom".to_string());
            } else if index <= SUPPORTED_MODEL_ENDPOINTS.len() {
                return Ok(SUPPORTED_MODEL_ENDPOINTS[index - 1].to_string());
            }
        }
        println!("Invalid choice. Please try again.");
    }
}

fn ask_yes_no(question: &str, default: bool) -> Result<bool, ApplicationError> {
    let default_option = if default { "Y/n" } else { "y/N" };
    print!("{} [{}]: ", question, default_option);
    io::stdout().flush()?;
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();
    Ok(match response.as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        "" => default,
        _ => ask_yes_no(question, default)?,
    })
}

fn path_to_tilde_string(path: &PathBuf) -> String {
    if let Ok(home_dir) = env::var("HOME") {
        let home_path = PathBuf::from(home_dir);
        if let Ok(relative_path) = path.strip_prefix(&home_path) {
            return format!("~/{}", relative_path.display());
        }
    }
    path.to_string_lossy().to_string()
}

fn get_project_directory() -> Result<String, ApplicationError> {
    let current_dir = env::current_dir()?;
    let tilde_current_dir = path_to_tilde_string(&current_dir);

    println!("Current directory:");
    println!("  {}", tilde_current_dir);

    print!("Enter project directory (or press Enter for current directory): ");
    io::stdout().flush()?;
    let mut dir = String::new();
    io::stdin().read_line(&mut dir)?;
    let dir = dir.trim();

    let path = if dir.is_empty() {
        current_dir.clone()
    } else {
        PathBuf::from(dir)
    };

    // Convert to absolute path
    let absolute_path = if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    };

    Ok(path_to_tilde_string(&absolute_path))
}

fn get_optional_value(key: &str) -> Result<Option<String>, ApplicationError> {
    print!(
        "Enter the value for '{}' (optional, press Enter to skip): ",
        key
    );
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim().to_string();
    Ok(if value.is_empty() { None } else { Some(value) })
}

fn get_secure_value(key: &str) -> Result<Option<String>, ApplicationError> {
    print!(
        "Enter the secure value for '{}' (optional, press Enter to skip): ",
        key
    );
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim().to_string();
    Ok(if value.is_empty() { None } else { Some(value) })
}

fn get_profile_name() -> Result<String, ApplicationError> {
    print!("Enter a name for the new profile: ");
    io::stdout().flush()?;
    let mut profile_name = String::new();
    io::stdin().read_line(&mut profile_name)?;
    Ok(profile_name.trim().to_string())
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

async fn setup_custom_encryption(
    db_handler: &mut UserProfileDbHandler,
    custom_ssh_key_path: &str,
) -> Result<(), ApplicationError> {
    println!("Setting up custom SSH key for encryption.");

    let key_path = PathBuf::from(custom_ssh_key_path);

    match EncryptionHandler::new_from_path(&key_path) {
        Ok(Some(handler)) => {
            db_handler.set_encryption_handler(Arc::new(handler))?;
        }
        Ok(None) => {
            return Err(ApplicationError::InvalidInput(
                "Failed to create encryption handler with the provided SSH key path.".to_string(),
            ));
        }
        Err(e) => {
            return Err(ApplicationError::InvalidInput(
                format!("Error registering SSH key: {}.", e),
            ));
        }
    }
    Ok(())
}
