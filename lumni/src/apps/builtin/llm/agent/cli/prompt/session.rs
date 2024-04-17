use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use bytes::Bytes;

use crate::external as lumni;
use lumni::HttpClient;

use super::responses::ChatCompletionResponse;

// temporary toggle to test streaming version
const STREAMING_RESPONSE: bool = true;

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
}

impl ChatSession {
    pub fn new(max_history: usize, instruction: String) -> ChatSession {
        ChatSession {
            http_client: HttpClient::new(),
            exchanges: Vec::new(),
            max_history,
            instruction,
        }
    }

    pub fn default() -> ChatSession {
        ChatSession::new(
            10,
            "A chat between a curious human and an artificial intelligence \
             assistant. The assistant gives helpful, detailed, and polite \
             answers to the human's questions"
                .to_string(),
        )
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        keep_running: Arc<AtomicBool>,
        message: String,
    ) {

        self.add_exchange(
            "Hello, Assistant.".to_string(),
            "Hello. How may I help you today?".to_string(),
        );
        self.add_exchange(
            "Please tell me the capital of France.".to_string(),
          "Sure. The capital of France is Paris".to_string(),
        );

        let initial_question = format!("Q: {}\nBot:", message);
        let prompt = self.create_final_prompt(message);

        if tx.send(
            ChatCompletionResponse::to_json_text(&initial_question).into(),
        ).await.is_err() {
            eprintln!("Receiver dropped");
            return;
        }
        let n_keep = 100;  // This should be calculated based on your needs
        let data_payload_result = self.create_payload(prompt, n_keep);

        let http_client = self.http_client.clone();

        tokio::spawn(async move {
            let data_payload = match data_payload_result {
                Ok(payload) => payload,
                Err(e) => format!("Failed to create payload: {}", e),
            };

            let header: HashMap<String, String> = [("Content-Type".to_string(), "application/json".to_string())].iter().cloned().collect();

            let payload_bytes = Bytes::from(data_payload.clone().into_bytes());
            let _ = http_client.post(
                // TODO: url should be passed as an argument
                "http://localhost:8080/completion",
                Some(&header),
                None,
                Some(&payload_bytes),
                if STREAMING_RESPONSE { Some(tx.clone()) } else { None },
            ).await;

            // Reset is_running after completion
            keep_running.store(false, Ordering::SeqCst);
        });
    }

    pub fn get_history(&self) -> &Vec<(String, String)> {
        &self.exchanges
    }

    pub fn add_exchange(&mut self, question: String, answer: String) {
        self.exchanges.push((question, answer));
        self.trim_history();
    }

    fn trim_history(&mut self) {
        if self.exchanges.len() / 2 > self.max_history {
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
            n_predict: 128,
            cache_prompt: true,
            stop: vec!["\n### Human:".to_string()],
            stream: STREAMING_RESPONSE,
        };

        serde_json::to_string(&payload)
    }
}

