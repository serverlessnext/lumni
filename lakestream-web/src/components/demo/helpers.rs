
#[cfg(feature = "debug-assertions")]
#[cfg(debug_assertions)]
pub async fn debug_sleep() {
    use std::time::Duration;
    use async_std::task;

    task::sleep(Duration::from_secs(1)).await;
}

