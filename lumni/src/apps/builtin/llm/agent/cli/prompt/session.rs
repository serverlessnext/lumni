use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bytes::Bytes;
use lumni::HttpClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::send::send_payload;
use super::PERSONAS;
use crate::external as lumni;

#[derive(Deserialize)]
struct TokenResponse {
    tokens: Vec<usize>,
}

#[derive(Debug, Deserialize)]
struct Persona {
    name: String,
    system_prompt: String,
    exchanges: Option<Vec<Exchange>>,
}

#[derive(Debug, Deserialize)]
struct Exchange {
    question: String,
    answer: String,
}

#[derive(Debug, Deserialize)]
struct Personas {
    personas: Vec<Persona>,
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
    exchanges: Vec<(String, String)>, // (question, answer)
    max_history: usize,
    instruction: String, // system
    n_keep: usize,
}

impl ChatSession {
    pub fn new() -> ChatSession {
        ChatSession {
            http_client: HttpClient::new(),
            exchanges: Vec::new(),
            max_history: 20, // TODO: base on max tokens
            instruction: "".to_string(),
            n_keep: 0,
        }
    }

    pub async fn init(&mut self) -> Result<(), Box<dyn Error>> {
        let selected_persona = "Default";
        let personas: Personas = serde_yaml::from_str(PERSONAS)?;

        // Find the selected persona by name
        if let Some(persona) = personas
            .personas
            .into_iter()
            .find(|p| p.name == selected_persona)
        {
            // Set session instruction from persona's system prompt
            self.instruction = persona.system_prompt;

            // Load predefined exchanges from persona if available
            if let Some(exchanges) = persona.exchanges {
                self.exchanges = exchanges.into_iter()
                    .map(|exchange| (exchange.question, exchange.answer))
                    .collect();
            }
        } else {
            return Err("Selected persona not found in the dataset".into());
        }

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

    pub async fn tokenize_and_set_n_keep(
        &mut self,
    ) -> Result<(), Box<dyn Error>> {
        let url = "http://localhost:8080/tokenize";
        let body_content =
            serde_json::json!({ "content": self.instruction }).to_string();
        let body = Bytes::from(body_content);

        let mut headers = HashMap::new();
        headers
            .insert("Content-Type".to_string(), "application/json".to_string());

        match self
            .http_client
            .post(url, Some(&headers), None, Some(&body), None)
            .await
        {
            Ok(http_response) => {
                match http_response.json::<TokenResponse>() {
                    Ok(response) => {
                        // Successfully parsed the token response, updating n_keep
                        self.n_keep = response.tokens.len();
                        log::debug!("n_keep set to {}", self.n_keep);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Failed to parse JSON response: {}", e);
                        Err(format!("Failed to parse JSON response: {}", e)
                            .into())
                    }
                }
            }
            Err(e) => Err(format!("HTTP request failed: {}", e).into()),
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
