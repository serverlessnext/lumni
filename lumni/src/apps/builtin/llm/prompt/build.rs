use std::env;
use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use regex::Regex;


fn generate_app_template(
    templates_path: &Path,
    output_path: &PathBuf,
) -> io::Result<()> {
    if !templates_path.exists() || !templates_path.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(&output_path)?;
    let output_file = output_path.join("templates.rs");

    let templates_regex = Regex::new(r"^.*\.yaml$").unwrap();
    let mut dest_file = File::create(output_file)?;

    // Process each YAML file within the templates directory
    for entry in fs::read_dir(templates_path)? {
        let entry = entry?;
        let file_path = entry.path();
        if templates_regex.is_match(file_path.to_str().unwrap()) {
            let mut contents = String::new();
            File::open(&file_path)?.read_to_string(&mut contents)?;
            let var_name = file_path
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('-', "_")
                .to_uppercase();
            writeln!(
                dest_file,
                "pub static {}: &str = r##\"{}\"##;",
                var_name,
                contents // No need to escape_default here
            )?;
        }
    }
    Ok(())
}

fn main() {

    let out_dir = match env::var("OUT_DIR") {
        Ok(dir) => dir,
        Err(e) => panic!("Failed to read OUT_DIR environment variable: {}", e),
    };
    // create a unique output directory for the generated code
    // this is required to avoid conflicts with other build scripts
    let output_path = Path::new(&out_dir).join("llm/prompt");

    if let Err(e) = generate_app_template(Path::new("./templates"), &output_path) {
        eprintln!("Error embedding templates: {}", e);
        std::process::exit(1);
    };
    // export OUTPUT_PATH so it be picked up by the main build script
    println!("cargo:warning=OUTPUT_PATH={}", output_path.to_str().unwrap());
}
