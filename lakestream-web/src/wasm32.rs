use wasm_bindgen::prelude::*;
use web_sys::console;

#[wasm_bindgen]
pub fn main() {
    console::log_1(&JsValue::from_str("Hello from Lakestream!"));

    wasm_bindgen_futures::spawn_local(async move {
        console::log_1(&JsValue::from_str("Inside future..."));
    });
    console::log_1(&JsValue::from_str("Future spawned."));
}
