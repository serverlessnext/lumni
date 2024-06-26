use std::io::{self, Write};
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::exchange::ChatExchange;
use super::history::ChatHistory;
use super::{LLMDefinition, PromptInstruction, ServerManager};
use crate::api::error::ApplicationError;

pub struct ChatSession {
    server: Box<dyn ServerManager>,
    prompt_instruction: PromptInstruction,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ChatSession {
    pub async fn new(
        mut server: Box<dyn ServerManager>,
        mut prompt_instruction: PromptInstruction,
        model: Option<LLMDefinition>,
    ) -> Result<Self, ApplicationError> {
        // initialize server directly if a model is available
        // otherwise, must be done from the terminal window
        if let Some(model) = model {
            server
                .setup_and_initialize(model, &mut prompt_instruction)
                .await?;
        }

        Ok(ChatSession {
            server,
            prompt_instruction,
            cancel_tx: None,
        })
    }

    pub fn stop(&mut self) {
        // Stop the chat session by sending a cancel signal
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    pub fn reset(&mut self) {
        self.stop();
        self.prompt_instruction.reset_history();
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        self.prompt_instruction.update_last_exchange(answer);
    }

    pub async fn finalize_last_exchange(
        &mut self,
        _tokens_predicted: Option<usize>,
    ) -> Result<(), ApplicationError> {
        // extract the last exchange, trim and tokenize it
        let token_length = if let Some(last_exchange) =
            self.prompt_instruction.get_last_exchange_mut()
        {
            // Strip off trailing whitespaces or newlines from the last exchange
            let trimmed_answer = last_exchange.get_answer().trim().to_string();
            last_exchange.set_answer(trimmed_answer);

            let temp_vec = vec![&*last_exchange];
            let model = self.server.get_selected_model()?;

            let last_prompt_text =
                ChatHistory::exchanges_to_string(model, temp_vec);

            if let Some(response) =
                self.server.tokenizer(&last_prompt_text).await?
            {
                Some(response.get_tokens().len())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(token_length) = token_length {
            if let Some(last_exchange) =
                self.prompt_instruction.get_last_exchange_mut()
            {
                last_exchange.set_token_length(token_length);
            }
        }

        Ok(())
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        question: String,
    ) -> Result<(), ApplicationError> {
        let max_token_length = self
            .server
            .get_context_size(&mut self.prompt_instruction)
            .await?;
        let new_exchange = self.initiate_new_exchange(question).await?;
        let n_keep = self.prompt_instruction.get_n_keep();
        let exchanges = self.prompt_instruction.new_prompt(
            new_exchange,
            max_token_length,
            n_keep,
        );

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancel_tx = Some(cancel_tx); // channel to cancel

        self.server
            .completion(
                &exchanges,
                &self.prompt_instruction,
                Some(tx),
                Some(cancel_rx),
            )
            .await?;
        Ok(())
    }

    pub async fn initiate_new_exchange(
        &self,
        user_question: String,
    ) -> Result<ChatExchange, ApplicationError> {
        let user_question = user_question.trim();
        let user_question = if user_question.is_empty() {
            "continue".to_string()
        } else {
            if let Some(prompt_template) =
                self.prompt_instruction.get_prompt_template()
            {
                prompt_template.replace("{{ USER_QUESTION }}", user_question)
            } else {
                user_question.to_string()
            }
        };

        let mut new_exchange = ChatExchange::new(user_question, "".to_string());
        let temp_vec = vec![&new_exchange];

        let model = self.server.get_selected_model()?;

        let last_prompt_text =
            ChatHistory::exchanges_to_string(model, temp_vec);

        if let Some(token_response) =
            self.server.tokenizer(&last_prompt_text).await?
        {
            new_exchange.set_token_length(token_response.get_tokens().len());
        }
        Ok(new_exchange)
    }

    pub fn process_response(
        &self,
        response: Bytes,
    ) -> (Option<String>, bool, Option<usize>) {
        self.server.process_response(response)
    }

    // used in non-interactive mode
    pub async fn process_prompt(
        &mut self,
        question: String,
        stop_signal: Arc<Mutex<bool>>,
    ) -> Result<(), ApplicationError> {
        let (tx, rx) = mpsc::channel(32);
        let _ = self.message(tx, question).await;
        self.handle_response(rx, stop_signal).await?;
        self.stop();
        Ok(())
    }

    async fn handle_response(
        &self,
        mut rx: mpsc::Receiver<Bytes>,
        stop_signal: Arc<Mutex<bool>>,
    ) -> Result<(), ApplicationError> {
        let mut final_received = false;
        while let Some(response) = rx.recv().await {
            // check if the session must be kept running
            if !*stop_signal.lock().await {
                log::debug!("Received stop signal");
                break;
            }

            if final_received {
                // consume stream until its empty, as server may send additional events
                // (e.g. stats, or logs) after the stop event.
                // for now these are ignored.
                continue;
            }
            let (response_content, is_final, _) =
                self.process_response(response);
            if let Some(response_content) = response_content {
                print!("{}", response_content);
            }
            io::stdout().flush().expect("Failed to flush stdout");

            if is_final {
                final_received = true;
            }
        }
        Ok(())
    }
}
