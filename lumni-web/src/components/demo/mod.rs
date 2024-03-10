mod dummy_data;
mod helpers;
mod load_and_submit;
mod load_form;

#[cfg(feature = "debug-assertions")]
#[macro_export]
macro_rules! debug_sleep {
    () => {
        super::helpers::debug_sleep().await;
    };
}
