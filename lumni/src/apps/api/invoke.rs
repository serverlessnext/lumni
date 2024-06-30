use futures::channel::mpsc;

use super::error::LumniError;
use crate::api::types::Data;
use crate::EnvironmentConfig;

#[derive(Debug)]
pub struct Request {
    content: Data,
    config: Option<EnvironmentConfig>,
    tx: mpsc::UnboundedSender<Result<Response, LumniError>>,
}

impl Request {
    pub fn new(
        content: Data,
        config: Option<EnvironmentConfig>,
        tx: mpsc::UnboundedSender<Result<Response, LumniError>>,
    ) -> Self {
        Self {
            content,
            config,
            tx,
        }
    }

    pub fn content(&self) -> &Data {
        &self.content
    }

    pub fn config(&self) -> Option<EnvironmentConfig> {
        self.config.clone()
    }

    pub fn tx(&self) -> &mpsc::UnboundedSender<Result<Response, LumniError>> {
        &self.tx
    }
}

#[derive(Debug)]
pub struct Response {
    content: Data,
}

impl Response {
    pub fn new(content: Data) -> Self {
        Self { content }
    }

    pub fn content(&self) -> &Data {
        &self.content
    }
}
