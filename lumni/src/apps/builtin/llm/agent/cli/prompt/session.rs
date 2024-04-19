use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::collections::HashMap;
use std::error::Error;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::external as lumni;
use lumni::HttpClient;

use super::send::send_payload;

#[derive(Deserialize)]
struct TokenResponse {
    tokens: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatPayload {
    prompt: String,
    temperature: f64,
    top_k: u32,
    top_p: f64,
    n_keep: usize,
    n_predict: u32,
    cache_prompt: bool,
    stop: Vec<String>,
    stream: bool,
}

pub struct ChatSession {
    http_client: HttpClient,
    exchanges: Vec<(String, String)>,
    max_history: usize,
    instruction: String,
    n_keep: usize,
}

impl ChatSession {
    pub fn new(max_history: usize, instruction: String) -> ChatSession {

        ChatSession {
            http_client: HttpClient::new(),
            exchanges: Vec::new(),
            max_history,
            instruction,
            n_keep: 0,
        }
    }

    pub fn default() -> ChatSession {
        let mut session = ChatSession::new(
            10,
            "A chat between a curious human and an artificial intelligence \
             assistant. The assistant gives helpful, detailed, and polite \
             answers to the human's questions"
                .to_string(),
        );
        session.add_exchange(
            "Hello, Assistant.".to_string(),
            "Hello. How may I help you today?".to_string(),
        );


        session.add_exchange(
            "Please tell me the capital of France.".to_string(),
            "Sure. The capital of France is Paris".to_string(),
        );
        session
    }

    pub async fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.tokenize_and_set_n_keep().await?;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.exchanges.clear();
    }

    pub fn update_last_exchange(&mut self, answer: String) {
        if let Some(last_exchange) = self.exchanges.last_mut() {
            last_exchange.1 = answer;
        }
    }


    pub async fn tokenize_and_set_n_keep(&mut self) -> Result<(), Box<dyn Error>> {
        let url = "http://localhost:8080/tokenize";
        let body_content = serde_json::json!({ "content": self.instruction }).to_string();
        let body = Bytes::from(body_content);
        
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        match self.http_client.post(url, Some(&headers), None, Some(&body), None).await {
            Ok(http_response) => {
                match http_response.json::<TokenResponse>() {
                    Ok(response) => {
                        // Successfully parsed the token response, updating n_keep
                        self.n_keep = response.tokens.len();
                        log::debug!("n_keep set to {}", self.n_keep);
                        Ok(())
                    },
                    Err(e) => {
                        eprintln!("Failed to parse JSON response: {}", e);
                        Err(format!("Failed to parse JSON response: {}", e).into())
                    }
                }
            },
            Err(e) => Err(format!("HTTP request failed: {}", e).into())
        }
    }


    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        keep_running: Arc<AtomicBool>,
        question: String,
    ) {
    
        let prompt = self.create_final_prompt(question.clone());
        // TODO: check if current last exchange has empty answer
        // if so -- overwrite it with the new question
        self.add_exchange(question, "".to_string());

        let n_keep = self.n_keep;
        let data_payload = self.create_payload(prompt, n_keep);

        log::debug!("Payload created:\n{:?}", data_payload);
        if let Ok(payload) = data_payload {
            send_payload(
                "http://localhost:8080/completion".to_string(),
                self.http_client.clone(),
                tx,
                payload,
                keep_running,
            )
            .await;
        }
    }

    pub fn get_history(&self) -> &Vec<(String, String)> {
        &self.exchanges
    }

    pub fn add_exchange(&mut self, question: String, answer: String) {
        self.exchanges.push((question, answer));
        self.trim_history();
    }

    fn trim_history(&mut self) {
        if (self.exchanges.len() / 2) > self.max_history {
            let excess = self.exchanges.len() - self.max_history * 2;
            self.exchanges.drain(0..excess);
        }
    }

    fn create_final_prompt(&self, question: String) -> String {
        let mut prompt = format!("{}\n", self.instruction);
        for (question, answer) in &self.exchanges {
            prompt.push_str(&format!(
                "\n### Human: {}\n### Assistant: {}",
                question, answer
            ));
        }
        prompt.push_str(&format!("\n### Human: {}", question));
        prompt
    }

    fn create_payload(
        &self,
        prompt: String,
        n_keep: usize,
    ) -> Result<String, serde_json::Error> {
        let payload = ChatPayload {
            prompt,
            temperature: 0.2,
            top_k: 40,
            top_p: 0.9,
            n_keep,
            n_predict: 4096,
            cache_prompt: true,
            stop: vec!["\n### Human:".to_string()],
            stream: true,
        };

        serde_json::to_string(&payload)
    }
}