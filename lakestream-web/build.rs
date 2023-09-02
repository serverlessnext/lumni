use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

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
    writeln!(f, "pub fn get_available_apps() -> Vec<&'static str> {{").unwrap();
    writeln!(f, "    vec![{}]", app_paths.join(", ")).unwrap();
    writeln!(f, "}}").unwrap();
}

fn traverse_and_generate(
    path: &Path,
    f: &mut File,
    app_paths: &mut Vec<String>,
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

        // Add path to the list
        app_paths.push(format!("\"{}\"", module_path));
    } else if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            traverse_and_generate(&entry.unwrap().path(), f, app_paths);
        }
    }
}
