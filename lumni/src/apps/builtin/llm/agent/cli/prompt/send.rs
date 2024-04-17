use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use bytes::Bytes;
use tokio::sync::mpsc;

use crate::external as lumni;
use lumni::HttpClient;
use super::responses::ChatCompletionResponse;

pub async fn send_payload(
    url: String,
    http_client: HttpClient,
    tx: mpsc::Sender<Bytes>,
    payload: String,
    keep_running: Arc<AtomicBool>,
) {
    let header = HashMap::from([(
        "Content-Type".to_string(),
        "application/json".to_string(),
    )]);

    let payload_bytes = Bytes::from(payload.into_bytes());

    tokio::spawn(async move {
        if let Err(e) = http_client
            .post(
                &url,
                Some(&header),
                None,
                Some(&payload_bytes),
                Some(tx.clone()),
            )
            .await
        {
            let error_message = format!(
                "{}",
                ChatCompletionResponse::to_json_text(&format!(
                    "HTTP Post error: {}",
                    e
                ))
            );
            tx.send(Bytes::from(error_message)).await.unwrap();
        }

        // Reset is_running after completion
        keep_running.store(false, Ordering::SeqCst);
    });
}