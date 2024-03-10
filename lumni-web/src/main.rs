use std::panic::{self, PanicInfo};

use leptos::{mount_to_body, view};
use lumni_web::app::App;

fn custom_panic_hook(info: &PanicInfo) {
    // print panic message only - not entire stack trace
    let message = info.to_string();
    log::error!("{}", message);
}

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    panic::set_hook(Box::new(custom_panic_hook));
    mount_to_body(|| view! { <App /> })
}
