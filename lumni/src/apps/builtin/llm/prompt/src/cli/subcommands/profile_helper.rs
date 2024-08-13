use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use dirs::home_dir;
use lumni::api::error::ApplicationError;
use serde_json::{json, Map, Value as JsonValue};

use super::{
    EncryptionHandler, ModelServer, ModelSpec, ServerTrait,
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
) -> Result<(), ApplicationError> {
    println!("Welcome to the profile creation wizard!");

    let profile_name = match &profile_name_to_update {
        Some(name) => name.clone(),
        None => get_profile_name()?,
    };
    db_handler.set_profile_name(profile_name.clone());

    let profile_type = select_profile_type()?;
    let mut settings = JsonValue::Object(Map::new());

    if profile_type != "Custom" {
        let model_server = ModelServer::from_str(&profile_type)?;

        if let Some(selected_model) = select_model(&model_server).await? {
            settings["MODEL_IDENTIFIER"] =
                JsonValue::String(selected_model.identifier.0);
        } else {
            println!("No model selected. Skipping model selection.");
        }

        // Get other predefined settings
        let server_settings = model_server.get_profile_settings();
        if let JsonValue::Object(map) = server_settings {
            for (key, value) in map {
                settings[key] = value;
            }
        }
    }

    if ask_yes_no("Do you want to set a project directory?", true)? {
        let dir = get_project_directory()?;
        settings["PROJECT_DIRECTORY"] = JsonValue::String(dir);
    }

    if profile_type == "Custom" {
        collect_custom_settings(&mut settings)?;
    } else {
        collect_profile_settings(&mut settings, &profile_type)?;
    }

    if ask_for_custom_ssh_key()? {
        setup_custom_encryption(db_handler).await?;
    }

    db_handler
        .create_or_update(&profile_name, &settings)
        .await?;

    println!(
        "Profile '{}' {} successfully!",
        profile_name,
        if profile_name_to_update.is_some() {
            "updated"
        } else {
            "created"
        }
    );

    Ok(())
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
                if let Err(e) = handle_not_ready() {
                    return Err(e);
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

fn handle_not_ready() -> Result<(), ApplicationError> {
    loop {
        print!(
            "Press Enter to retry, 'q' to quit, or 's' to skip model \
             selection: "
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "" => return Ok(()),
            "q" => {
                return Err(ApplicationError::UserCancelled(
                    "Model selection cancelled by user".to_string(),
                ))
            }
            "s" => return Ok(()),
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

fn collect_custom_settings(
    settings: &mut JsonValue,
) -> Result<(), ApplicationError> {
    loop {
        print!("Enter a custom key (or press Enter to finish): ");
        io::stdout().flush()?;
        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        if key.is_empty() {
            break;
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

fn collect_profile_settings(
    settings: &mut JsonValue,
    profile_type: &str,
) -> Result<(), ApplicationError> {
    if let JsonValue::Object(ref mut map) = settings {
        let mut updates = Vec::new();
        let mut removals = Vec::new();

        for (key, value) in map.iter() {
            if *value == JsonValue::Null {
                if let Some(new_value) = get_optional_value(key)? {
                    updates.push((key.clone(), JsonValue::String(new_value)));
                } else {
                    removals.push(key.clone());
                }
            } else if let JsonValue::Object(obj) = value {
                if obj.contains_key("content")
                    && obj.contains_key("encryption_key")
                {
                    if let Some(new_value) = get_secure_value(key)? {
                        updates.push((
                            key.clone(),
                            json!({
                                "content": new_value,
                                "encryption_key": "",
                            }),
                        ));
                    } else {
                        removals.push(key.clone());
                    }
                }
            }
        }

        // Apply updates
        for (key, value) in updates {
            map.insert(key, value);
        }

        // Apply removals
        for key in removals {
            map.remove(&key);
        }
    }

    if profile_type == "Custom" {
        loop {
            print!("Enter a custom key (or press Enter to finish): ");
            io::stdout().flush()?;
            let mut key = String::new();
            io::stdin().read_line(&mut key)?;
            let key = key.trim();

            if key.is_empty() {
                break;
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
    }

    Ok(())
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
