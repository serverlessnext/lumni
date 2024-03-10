use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AppSpec {
    app_info: AppInfo,
}

#[derive(Debug, Deserialize)]
struct AppInfo {
    name: String,
    display_name: String,
    version: String,
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_modules.rs");
    let mut f = File::create(&dest_path).unwrap();

    writeln!(
        f,
        "pub fn get_app_handler(app_uri: &str) -> Option<Box<dyn AppHandler>> \
         {{"
    )
    .unwrap();
    writeln!(f, "    match app_uri {{").unwrap();

    let mut app_paths = Vec::new();
    let apps_dir = Path::new("src/apps/builtin");
    traverse_and_generate(&apps_dir, &mut f, &mut app_paths);

    writeln!(f, "        _ => None,").unwrap();
    writeln!(f, "    }}").unwrap(); // Closing brace for the match statement
    writeln!(f, "}}").unwrap(); // Closing brace for get_app_handler

    // Generate function to get available apps
    writeln!(f, "use std::collections::HashMap;\n").unwrap();
    writeln!(
        f,
        "pub fn get_available_apps() -> Vec<HashMap<String, String>> {{"
    )
    .unwrap();
    writeln!(f, "    vec![{}]", generate_app_strings(&app_paths)).unwrap();
    writeln!(f, "}}").unwrap();
}

fn traverse_and_generate(
    path: &Path,
    f: &mut File,
    app_paths: &mut Vec<HashMap<String, String>>,
) {
    if path.is_dir() && path.join("handler.rs").exists() {
        let module_path = path
            .strip_prefix("src/apps/")
            .unwrap()
            .to_str()
            .unwrap()
            .replace("/", "::");
        writeln!(
            f,
            "        \"{}\" => Some(Box::new(crate::apps::{}::Handler)),",
            module_path, module_path
        )
        .unwrap();

        // Parse app_info from spec.yaml
        let spec_path = path.join("spec.yaml");
        if spec_path.exists() {
            let content = std::fs::read_to_string(&spec_path).unwrap();
            match serde_yaml::from_str::<AppSpec>(&content) {
                Ok(app_spec) => {
                    let mut app_info_map = HashMap::new();
                    app_info_map
                        .insert("name".to_string(), app_spec.app_info.name);
                    app_info_map.insert(
                        "display_name".to_string(),
                        app_spec.app_info.display_name,
                    );
                    app_info_map.insert(
                        "version".to_string(),
                        app_spec.app_info.version,
                    );
                    app_info_map
                        .insert("__uri__".to_string(), module_path.clone());
                    app_paths.push(app_info_map);
                }
                Err(e) => {
                    eprintln!(
                        "Failed to parse spec.yaml in {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            traverse_and_generate(&entry.unwrap().path(), f, app_paths);
        }
    }
}

fn generate_app_strings(app_paths: &Vec<HashMap<String, String>>) -> String {
    app_paths
        .iter()
        .map(|app| {
            let pairs: Vec<String> = app
                .iter()
                .map(|(k, v)| {
                    format!(
                        "map.insert(\"{}\".to_string(), \"{}\".to_string());",
                        k, v
                    )
                })
                .collect();
            format!(
                "{{\n    let mut map = HashMap::new();\n    {}\n    map\n}}",
                pairs.join("\n    ")
            )
        })
        .collect::<Vec<String>>()
        .join(", ")
}
