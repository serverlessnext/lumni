

use crate::{
    LakestreamError,
    BinaryCallbackWrapper,
};
use crate::http::HttpClient;

pub struct HttpHandler {
    client: HttpClient,
    callback: Option<BinaryCallbackWrapper>,
}

impl HttpHandler {
    pub fn new(callback: Option<BinaryCallbackWrapper>) -> Self {
        Self {
            client: HttpClient::new(),
            callback: callback,
        }
    }

    pub async fn get(&self, url: &str) -> Result<Option<Vec<u8>>, LakestreamError> {
        let response = self.client.get(url, None, None).await.map_err(LakestreamError::from)?;
        let data = response.body(); 

        if self.callback.is_some() {
            if let Some(data) = &response.body() {
                if let Some(callback) = &self.callback {
                    let _ = callback.call(data.to_vec()).await;
                }
            }
            Ok(None)
        } else {
            Ok(data.map(|b| b.to_vec()))
        }
    }
}