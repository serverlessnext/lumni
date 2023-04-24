
use lakestream_web::{App, AppProps};

use leptos::{log, mount_to_body, view};

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    log!("Hello from Lakestream console!");
    mount_to_body(|cx| view! { cx, <App /> })
}
