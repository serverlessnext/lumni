use std::collections::HashMap;
use bytes::{Bytes, BytesMut};
use lumni::api::error::{HttpClientError, ApplicationError};
use lumni::HttpClient;
use tokio::sync::{mpsc, oneshot};

pub use crate::external as lumni;

pub async fn http_post(
    url: String,
    http_client: HttpClient,
    tx: Option<mpsc::Sender<Bytes>>,
    payload: String,
    http_headers: Option<HashMap<String, String>>,
    cancel_rx: Option<oneshot::Receiver<()>>,
) {
    let headers = if let Some(http_headers) = http_headers {
        http_headers
    } else {
        HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])
    };

    let payload_bytes = Bytes::from(payload.into_bytes());
    tokio::spawn(async move {
        match http_client
            .post(
                &url,
                Some(&headers),
                None,
                Some(&payload_bytes),
                tx.clone(),
                cancel_rx,
            )
            .await
        {
            Err(HttpClientError::RequestCancelled) => {} // request cancelled by user
            Err(e) => log::error!("HTTP Post error: {}", e),
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
            Err(e) => log::error!("HTTP Get error: {}", e),
            Ok(_) => {}
        }
    });
}

pub async fn http_get_with_response(
    url: String,
    http_client: HttpClient,
) -> Result<Bytes, ApplicationError> {
    let header = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);
    let (tx, mut rx) = mpsc::channel(1);
    
    let result = http_client.get(&url, Some(&header), None, Some(tx), None).await;

    match result {
        Ok(_) => {
            let mut response_bytes = BytesMut::new();
            while let Some(response) = rx.recv().await {
                response_bytes.extend_from_slice(&response);
            }
            drop(rx); // drop the receiver to close the channel
            Ok(response_bytes.freeze())
        },
        Err(e) => Err(e.into())
    }
}

pub async fn http_post_with_response(
    url: String,
    http_client: HttpClient,
    payload: String,
) -> Result<Bytes, ApplicationError> {
    let headers = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);
    let (tx, mut rx) = mpsc::channel(1);
    let payload_bytes = Bytes::from(payload.into_bytes());

    let result = http_client.post(&url, Some(&headers), None, Some(&payload_bytes), Some(tx), None).await;

    // Handle the result of the HTTP POST request
    match result {
        Ok(_) => {
            let mut response_bytes = BytesMut::new();
            while let Some(response) = rx.recv().await {
                response_bytes.extend_from_slice(&response);
            }
            drop(rx); // drop the receiver to close the channel
            Ok(response_bytes.freeze())
        },
        Err(e) => Err(e.into())
    }
}

