use std::collections::HashMap;

use bytes::{Bytes, BytesMut};
use lumni::api::error::{ApplicationError, HttpClientError};
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

pub async fn http_get_with_response(
    url: String,
    http_client: HttpClient,
) -> Result<Bytes, ApplicationError> {
    let header = HashMap::from([(
        "Content-Type".to_string(),
        "application/json".to_string(),
    )]);
    let (tx, mut rx) = mpsc::channel(1);
    
    // Spawn a task to handle the HTTP request
    let request_task = tokio::spawn(async move {
        http_client.get(&url, Some(&header), None, Some(tx), None).await
    });

    let mut response_bytes = BytesMut::new();
    
    // Receive chunks from the channel
    while let Some(response) = rx.recv().await {
        response_bytes.extend_from_slice(&response);
    }

    // Wait for the request task to complete
    let result = request_task.await?;

    // Handle the result
    match result {
        Ok(_) => Ok(response_bytes.freeze()),
        Err(e) => Err(e.into()),
    }
}

pub async fn http_post_with_response(
    url: String,
    http_client: HttpClient,
    payload: String,
) -> Result<Bytes, ApplicationError> {
    let headers = HashMap::from([(
        "Content-Type".to_string(),
        "application/json".to_string(),
    )]);
    let (tx, mut rx) = mpsc::channel(1);
    let payload_bytes = Bytes::from(payload.into_bytes());
    
    // Spawn a task to handle the HTTP request
    let request_task = tokio::spawn(async move {
        http_client.post(
            &url,
            Some(&headers),
            None,
            Some(&payload_bytes),
            Some(tx),
            None,
        ).await
    });

    let mut response_bytes = BytesMut::new();
    
    // Receive chunks from the channel
    while let Some(response) = rx.recv().await {
        response_bytes.extend_from_slice(&response);
    }

    // Wait for the request task to complete
    let result = request_task.await?;

    // Handle the result
    match result {
        Ok(_) => Ok(response_bytes.freeze()),
        Err(e) => Err(e.into()),
    }
}
