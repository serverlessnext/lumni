mod load_and_submit;
mod load_form;
mod dummy_data;
mod helpers;

pub use load_form::LoadFormDemo;
pub use load_and_submit::LoadAndSubmitDemo;


#[cfg(feature = "debug-assertions")]
#[macro_export]
macro_rules! debug_sleep {
    () => {
        super::helpers::debug_sleep().await;
    };
}
