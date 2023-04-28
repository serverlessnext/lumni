use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

const DEFAULT_VERSION: &str = "0.0.0";

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");
    // update version in Cargo.toml for all crates in repo
    if let Ok(version) = env::var("BUILD_VERSION") {

        // if version is empty, set default to 0.0.0
        let version = if version.is_empty() {
            DEFAULT_VERSION.to_string()
        } else {
            version
        };

        let crates = &[
            "lakestream",
            "lakestream-cli",
            "lakestream-py",
            "lakestream-web",
        ];

        for crate_name in crates {
            let path = Path::new("..").join(crate_name).join("Cargo.toml");
            let mut contents = String::new();
            let mut file = File::open(&path).expect("Unable to open file");
            file.read_to_string(&mut contents)
                .expect("Unable to read file contents");

            let mut doc = contents.parse::<toml_edit::Document>().expect("Invalid TOML");
            doc["package"]["version"] = toml_edit::value(version.clone());
            let output = doc.to_string_in_original_order();

            let mut file = File::create(&path).expect("Unable to create file");
            file.write_all(output.as_bytes())
                .expect("Unable to write to file");
        }
    }
}
