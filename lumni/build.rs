use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::io::{self, Read, Write, ErrorKind};
use std::path::Path;

use std::process::Command;
use regex::Regex;
use serde::Deserialize;

const DEFAULT_VERSION: &str = "0.0.0";
const APPS_DIRECTORY: &str = "src/apps/builtin";

#[derive(Debug, Deserialize)]
struct ApplicationSpec {
    package: Option<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    display_name: String,
    version: String,
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    // Recursively copies a directory from a source to a destination.
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn copy_files_to_target(source_path: &Path, target_path: &Path) -> io::Result<()> {
    if source_path.exists() {
        copy_dir_all(source_path, target_path)
    } else {
        Err(io::Error::new(ErrorKind::NotFound, "Source path does not exist"))
    }
}

fn update_build_version() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");
    // update version in Cargo.toml for all crates in repo
    if let Ok(version) = env::var("BUILD_VERSION") {
        // if version is empty, set default to 0.0.0
        let version = if version.is_empty() {
            DEFAULT_VERSION.to_string()
        } else {
            version
        };

        let crates = &["lumni", "lumni-py", "lumni-web"];

        for crate_name in crates {
            let path = Path::new("..").join(crate_name).join("Cargo.toml");
            let mut contents = String::new();
            let mut file = File::open(&path).expect("Unable to open file");
            file.read_to_string(&mut contents)
                .expect("Unable to read file contents");

            let mut doc = contents
                .parse::<toml_edit::Document>()
                .expect("Invalid TOML");
            doc["package"]["version"] = toml_edit::value(version.clone());
            let output = doc.to_string();

            let mut file = File::create(&path).expect("Unable to create file");
            file.write_all(output.as_bytes())
                .expect("Unable to write to file");
        }
    }
}

// Generate a function to get app handler based on app_uri
fn generate_app_handler() {
    println!("cargo:rerun-if-changed={}", APPS_DIRECTORY); // Trigger on changes
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
    let apps_dir = Path::new(APPS_DIRECTORY);

    // Compile regex patterns outside of the loop for efficiency
    let name_pattern = Regex::new(r"^[-a-z0-9]*$").unwrap();
    let uri_pattern = Regex::new(r"^[-a-z]+::[-a-z0-9]+::[-a-z0-9]+$").unwrap();
    let version_pattern = Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").unwrap();

    traverse_and_generate(
        &apps_dir,
        &mut f,
        &mut app_paths,
        &name_pattern,
        &uri_pattern,
        &version_pattern,
    );

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
    name_pattern: &Regex,
    uri_pattern: &Regex,
    version_pattern: &Regex,
) {
    if path.is_dir() && path.join("src/handler.rs").exists() {
        let module_path = path
            .strip_prefix("src/apps/")
            .unwrap()
            .to_str()
            .unwrap()
            .replace("/", "::");
        writeln!(
            f,
            "        \"{}\" => Some(Box::new(crate::apps::{}::src::Handler)),",
            module_path, module_path
        )
        .unwrap();

        // Parse app_info from spec.yaml
        let spec_path = path.join("spec.yaml");
        if spec_path.exists() {
            let content = std::fs::read_to_string(&spec_path).unwrap();
            match serde_yaml::from_str::<ApplicationSpec>(&content) {
                Ok(app_spec) => {
                    let package = app_spec.package.unwrap();

                    // Validate name
                    if !name_pattern.is_match(&package.name.to_lowercase()) {
                        panic!("Invalid name pattern for '{}'", package.name);
                    }

                    // Validate __uri__
                    if !uri_pattern.is_match(&module_path) {
                        panic!("Invalid __uri__ pattern for '{}'", module_path);
                    }

                    // Validate version
                    if !version_pattern.is_match(&package.version) {
                        panic!(
                            "Invalid version pattern for '{}'",
                            package.version
                        );
                    }

                    let mut app_info_map = HashMap::new();
                    app_info_map.insert(
                        "name".to_string(),
                        package.name.to_lowercase(),
                    );
                    app_info_map.insert(
                        "display_name".to_string(),
                        package.display_name,
                    );
                    app_info_map.insert("version".to_string(), package.version);
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
            traverse_and_generate(
                &entry.unwrap().path(),
                f,
                app_paths,
                name_pattern,
                uri_pattern,
                version_pattern,
            );
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


fn traverse_and_invoke(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Recursively look for build.rs files
                traverse_and_invoke(&path)?;
            } else if path.file_name().unwrap() == "build.rs" {
                // Compile and execute the custom build.rs found
                run_app_build_script(&path)?;
            }
        }
    }
    Ok(())
}

fn run_app_build_script(build_script_path: &Path) -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();

    let app_dir = build_script_path.parent().ok_or_else(|| 
        io::Error::new(ErrorKind::Other, "Failed to get parent directory of build script"))?;

    let relative_path = build_script_path.parent().unwrap()
        .strip_prefix(Path::new(APPS_DIRECTORY))
        .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to calculate relative path"))?;
    let app_target_path = Path::new(&out_dir).join(relative_path);

    // Create target directory
    fs::create_dir_all(&app_target_path)?;

    // Compile the build script in the app directory
    compile_build_script(&app_dir)?;

    // Extract output path and copy files
    let app_output_path = extract_output_path(&app_dir)?;
    copy_files_to_target(&app_output_path, &app_target_path)
}

fn compile_build_script(app_dir: &Path) -> io::Result<()> {
    let compile = Command::new("cargo")
        .env("CARGO_TARGET_DIR", app_dir)
        .arg("build")
        .current_dir(app_dir)
        .output()?;

    if !compile.status.success() {
        eprintln!("Compilation errors: {}", String::from_utf8_lossy(&compile.stderr));
        return Err(io::Error::new(ErrorKind::Other, "Failed to compile build script."));
    }
    Ok(())
}

fn extract_output_path(app_dir: &Path) -> io::Result<PathBuf> {
    let compile_output = Command::new("cargo")
        .env("CARGO_TARGET_DIR", env::var("OUT_DIR").unwrap())
        .arg("build")
        .current_dir(app_dir)
        .output()?;

    let output_str = String::from_utf8_lossy(&compile_output.stderr);
    let regex = Regex::new(r"OUTPUT_PATH=([^\s]+)").unwrap();
    regex.captures(&output_str)
        .and_then(|caps| caps.get(1).map(|m| PathBuf::from(m.as_str())))
        .ok_or_else(|| io::Error::new(ErrorKind::Other, "Failed to extract output path from build output"))
}



fn main() {
    update_build_version();
    generate_app_handler();

    if let Err(e) = traverse_and_invoke(Path::new(APPS_DIRECTORY)) {
        eprintln!("Error invoking build.rs: {}", e);
    }
}