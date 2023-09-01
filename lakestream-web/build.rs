use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_modules.rs");
    let mut f = File::create(&dest_path).unwrap();

    writeln!(f, "use crate::api::handler::AppHandler;").unwrap();

    writeln!(
        f,
        "pub fn get_app_handler(app_name: &str) -> Option<Box<dyn \
         AppHandler>> {{"
    )
    .unwrap();
    writeln!(f, "    match app_name {{").unwrap();

    let apps_dir = Path::new("src/apps/builtin");
    traverse_and_generate(&apps_dir, &mut f);

    writeln!(f, "        _ => None,").unwrap();
    writeln!(f, "    }}").unwrap();
    writeln!(f, "}}").unwrap();
}

fn traverse_and_generate(path: &Path, f: &mut File) {
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
    } else if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            traverse_and_generate(&entry.unwrap().path(), f);
        }
    }
}
