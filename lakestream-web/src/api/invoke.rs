use futures::channel::mpsc;
use lakestream::EnvironmentConfig;

use super::error::*;
use crate::api::types::Data;

#[derive(Debug)]
pub struct Request {
    content: Data,
    config: Option<EnvironmentConfig>,
    tx: mpsc::UnboundedSender<Result<Response, Error>>,
}

impl Request {
    pub fn new(
        content: Data,
        config: Option<EnvironmentConfig>,
        tx: mpsc::UnboundedSender<Result<Response, Error>>,
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

    pub fn tx(&self) -> &mpsc::UnboundedSender<Result<Response, Error>> {
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

    pub fn is_empty(&self) -> bool {
        match &self.content {
            Data::Empty => true,
            _ => false,
        }
    }

    pub fn content(&self) -> &Data {
        &self.content
    }
}
