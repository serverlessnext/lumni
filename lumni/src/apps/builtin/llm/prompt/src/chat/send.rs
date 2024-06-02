use std::collections::HashMap;

use bytes::Bytes;
use lumni::api::error::HttpClientError;
use lumni::HttpClient;
use tokio::sync::{mpsc, oneshot};

use super::responses::ChatCompletionResponse;
use crate::external as lumni;

pub async fn send_payload(
    url: String,
    http_client: HttpClient,
    tx: Option<mpsc::Sender<Bytes>>,
    payload: String,
    cancel_rx: Option<oneshot::Receiver<()>>,
) {
    let header = HashMap::from([(
        "Content-Type".to_string(),
        "application/json".to_string(),
    )]);
    let payload_bytes = Bytes::from(payload.into_bytes());
    tokio::spawn(async move {
        match http_client
            .post(
                &url,
                Some(&header),
                None,
                Some(&payload_bytes),
                tx.clone(),
                cancel_rx,
            )
            .await
        {
            Err(HttpClientError::RequestCancelled) => {} // request cancelled by user
            Err(e) => {
                eprintln!("An error occurred: {}", e);
                let error_message = format!(
                    "{}",
                    ChatCompletionResponse::to_json_text(&format!(
                        "HTTP Post error: {}",
                        e
                    ))
                );
                if let Some(tx) = tx {
                    tx.send(Bytes::from(error_message)).await.unwrap();
                };
            }
            Ok(_) => {} // request successful
        }
    });
}
