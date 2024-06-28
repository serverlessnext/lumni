use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use lumni::{
    AWSCredentials, AWSRequestBuilder, HttpClient, HttpClientError,
    HttpClientErrorHandler, HttpClientResponse,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use super::{
    http_post, ChatExchange, ChatHistory, ChatMessage, Endpoints,
    LLMDefinition, PromptInstruction, ServerTrait,
};
pub use crate::external as lumni;

struct AWSErrorHandler;

impl HttpClientErrorHandler for AWSErrorHandler {
    fn handle_error(
        &self,
        response: HttpClientResponse,
        canonical_reason: String,
    ) -> HttpClientError {
        if response.status_code() == 403 {
            if let Some(value) = response.headers().get("x-amzn-errortype") {
                if let Ok(err_type) = value.to_str() {
                    if err_type.starts_with("ExpiredTokenException") {
                        return HttpClientError::HttpError(
                            403,
                            "ExpiredToken".to_string(),
                        );
                    }
                }
            }
        }
        // Fallback if no special handling is needed
        HttpClientError::HttpError(response.status_code(), canonical_reason)
    }
}

pub struct Bedrock {
    http_client: HttpClient,
    endpoints: Endpoints,
    model: Option<LLMDefinition>,
}

impl Bedrock {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // TODO: get region from AWSCredentials
        let bedrock_endpoint = "https://bedrock-runtime.us-east-1.amazonaws.com";
        let endpoints = Endpoints::new()
            .set_completion(Url::parse(bedrock_endpoint)?)
            .set_list_models(Url::parse(bedrock_endpoint)?);

        Ok(Bedrock {
            http_client: HttpClient::new()
                .with_error_handler(Arc::new(AWSErrorHandler)),
            endpoints,
            model: None,
        })
    }
}

#[async_trait]
impl ServerTrait for Bedrock {
    async fn initialize_with_model(
        &mut self,
        model: LLMDefinition,
        _prompt_instruction: &PromptInstruction,
    ) -> Result<(), Box<dyn Error>> {
        self.model = Some(model);
        Ok(())
    }

    fn get_model(&self) -> Option<&LLMDefinition> {
        self.model.as_ref()
    }

    fn process_response(
        &self,
        response: &Bytes,
    ) -> (String, bool, Option<usize>) {
        // convert response to string
        let response_str = String::from_utf8_lossy(response);
        //eprintln!("Response: {:?}", response_str);
        let response_length = response_str.len().to_string();
        (response_length, true, None)
        //        match BedrockCompletionResponse::extract_content(response) {
        //            Ok(chat) => (chat.message.content, chat.done, chat.eval_count),
        //            Err(e) => (format!("Failed to parse JSON: {}", e), true, None),
        //        }
    }

    async fn completion(
        &self,
        exchanges: &Vec<ChatExchange>,
        prompt_instruction: &PromptInstruction,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> Result<(), Box<dyn Error>> {
        let system_prompt = prompt_instruction.get_instruction();
        let model = self.model.as_ref().expect("Model not available");

        let resource = HttpClient::percent_encode_with_exclusion(
            &format!("/model/{}/converse-stream", model.get_name()),
            Some(&[b'/', b'.', b'-']),
        );
        let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        let full_url = format!("{}{}", completion_endpoint, resource);

        let payload = "{\"messages\": [{\"role\": \"user\", \"content\": \
            \"".to_string();
        let payload_hash = Sha256::digest(payload.as_bytes())
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();
        let request_builder = AWSRequestBuilder::new(completion_endpoint);
        let credentials = AWSCredentials::from_env()
            .ok()
            .expect("No credentials found");
        let headers = request_builder.generate_headers(
            "POST",
            "bedrock",
            &credentials,
            // resource must be double percent encoded, generate_headers() will percent encode
            // it again. I.e. double percent-encoded, required to get correct v4 sig
            Some(&resource),
            None, // query string
            Some(&payload_hash),
        )?;
        //let completion_endpoint = self.endpoints.get_completion_endpoint()?;
        http_post(
            full_url,
            self.http_client.clone(),
            tx,
            payload,
            Some(headers),
            cancel_rx,
        )
        .await;
        Ok(())
    }

    async fn list_models(
        &self,
    ) -> Result<Option<Vec<LLMDefinition>>, Box<dyn Error>> {
        let model = LLMDefinition::new("anthropic.claude-3-5-sonnet-20240620-v1:0".to_string());
        Ok(Some(vec![model]))
    }
}
