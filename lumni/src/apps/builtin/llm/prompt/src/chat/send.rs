use std::collections::HashMap;
use std::error::Error;

use bytes::{Bytes, BytesMut};
use lumni::api::error::HttpClientError;
use lumni::HttpClient;
use tokio::sync::{mpsc, oneshot};

use super::responses::ChatCompletionResponse;
use crate::external as lumni;

pub async fn http_post(
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

pub async fn http_get(
    url: String,
    http_client: HttpClient,
    tx: Option<mpsc::Sender<Bytes>>,
    cancel_rx: Option<oneshot::Receiver<()>>,
) {
    let header = HashMap::from([(
        "Content-Type".to_string(),
        "application/json".to_string(),
    )]);
    tokio::spawn(async move {
        match http_client
            .get(&url, Some(&header), None, tx.clone(), cancel_rx)
            .await
        {
            Err(HttpClientError::RequestCancelled) => {} // request cancelled by user
            Err(e) => {
                eprintln!("An error occurred: {}", e);
                let error_message = format!(
                    "{}",
                    ChatCompletionResponse::to_json_text(&format!(
                        "HTTP Get error: {}",
                        e
                    ))
                );
                if let Some(tx) = tx {
                    tx.send(Bytes::from(error_message)).await.unwrap();
                };
            }
            Ok(_) => {}
        }
    });
}

pub async fn http_get_with_response(
    url: String,
    http_client: HttpClient,
) -> Result<Bytes, Box<dyn Error>> {
    let mut response_bytes = BytesMut::new();
    let (tx, mut rx) = mpsc::channel(1);
    http_get(url, http_client, Some(tx), None).await;

    while let Some(response) = rx.recv().await {
        response_bytes.extend_from_slice(&response);
    }
    drop(rx); // drop the receiver to close the channel
    Ok(response_bytes.freeze())
}
