use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

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
    exchanges: Vec<(String, String)>,
    max_history: usize,
    instruction: String,
}

impl ChatSession {
    pub fn new(max_history: usize, instruction: String) -> ChatSession {
        ChatSession {
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
        &self,
        tx: mpsc::Sender<String>,
        keep_running: Arc<AtomicBool>,
        message: String,
    ) {
        tokio::spawn(async move {
            // First, send the initial formatted question
            let initial_question = format!("Q: {}\nBot:", message);
            if tx.send(initial_question).await.is_err() {
                println!("Receiver dropped");
                return;
            }

            // Words to simulate a bot's streaming response
            let response_words = vec![
                "some",
                "random",
                "answer",
                "to",
                "simulate",
                "a",
                "streaming",
                "response",
                "from",
                "a",
                "bot",
            ];

            // Stream each word one by one
            for word in response_words {
                if !keep_running.load(Ordering::SeqCst) {
                    break; // Stop sending if is_running is set to false
                }
                let mut response_text = String::from("");
                response_text.push(' '); // Add a space before each word
                response_text.push_str(word); // Append the word to the ongoing sentence

                if tx.send(response_text.clone()).await.is_err() {
                    println!("Receiver dropped");
                    return;
                }
            }

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

    fn create_final_prompt(&self) -> String {
        let mut prompt = format!("{}\n", self.instruction);
        for (question, answer) in &self.exchanges {
            prompt.push_str(&format!(
                "\n### Human: {}\n### Assistant: {}",
                question, answer
            ));
        }
        prompt
    }

    fn create_payload(
        &self,
        n_keep: usize,
    ) -> Result<String, serde_json::Error> {
        let prompt = self.create_final_prompt();

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

pub async fn run_prompt() -> Result<(), Box<dyn Error>> {
    let instruction = "A chat between a curious human and an artificial \
                       intelligence assistant. The assistant gives helpful, \
                       detailed, and polite answers to the human's questions."
        .to_string();
    let mut chat = ChatSession::new(10, instruction);

    chat.add_exchange(
        "Hello, Assistant.".to_string(),
        "Hello. How may I help you today?".to_string(),
    );
    chat.add_exchange(
        "Please tell me the capital of France.".to_string(),
        "Sure. The capital of France is Paris".to_string(),
    );

    // TODO: add token count
    let n_keep = 100;

    // Create the payload for the API request
    let data_payload = chat.create_payload(n_keep)?;
    println!("Data payload for API request: {}", data_payload);

    // TODO: integrate with POST request
    // stream data back

    Ok(())
}
