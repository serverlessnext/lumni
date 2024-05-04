
// This file is a dummy file to test the template generation.

include!(concat!(env!("OUT_DIR"), "/llm/prompt/templates.rs"));

fn main() {
    eprintln!("Template: {:?}", PERSONAS);
}
