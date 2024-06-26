use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};

use super::exchange::ChatExchange;
use super::history::ChatHistory;
use super::{LLMDefinition, ModelServer, PromptInstruction, ServerTrait};

pub struct ChatSession {
    history: ChatHistory,
    server: Box<dyn ServerTrait>,
    model: Option<LLMDefinition>,
    prompt_instruction: PromptInstruction,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl ChatSession {
    pub fn new(
        server_name: String,
        options: Option<&String>,
    ) -> Result<Self, Box<dyn Error>> {
        let server = Box::new(ModelServer::from_str(&server_name)?);

        let mut prompt_instruction = PromptInstruction::default();
        if let Some(json_str) = options {
            prompt_instruction
                .get_prompt_options_mut()
                .update_from_json(json_str);
            prompt_instruction
                .get_completion_options_mut()
                .update_from_json(json_str);
        }

        Ok(ChatSession {
            history: ChatHistory::new(),
            server,
            model: None,
            prompt_instruction,
            cancel_tx: None,
        })
    }

    pub async fn init(
        &mut self,
        instruction: Option<String>,
        assistant: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        // If both instruction and assistant are None, use the default assistant
        let assistant = if instruction.is_none() && assistant.is_none() {
            // for useful responses, there should either be a system prompt or an
            // assistant set. If none are given use the default assistant.
            Some("Default".to_string())
        } else {
            assistant
        };

        if let Some(assistant) = assistant {
            self.prompt_instruction.preload_from_assistant(
                assistant,
                &mut self.history,
                instruction, // add user-instruction with assistant
            )?;
        } else if let Some(instruction) = instruction {
            self.prompt_instruction.set_system_prompt(instruction);
        };
        // set token length for the system prompt
        let instruction = self.prompt_instruction.get_instruction();
        if instruction.is_empty() {
            self.prompt_instruction.set_system_token_length(Some(0));
        } else {
            self.prompt_instruction.set_system_token_length(
                self.server.token_length(instruction).await?,
            );
        };

        self.model = self.server.get_model().await?;

        if let Some(model) = self.model.as_ref() {
            self.prompt_instruction
                .get_completion_options_mut()
                .update_from_model(model);
        }

        self.server
            .initialize(self.model.as_ref(), &mut self.prompt_instruction)
            .await?;

        Ok(())
    }

    pub fn stop(&mut self) {
        // Stop the chat session by sending a cancel signal
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }

    pub fn reset(&mut self) {
        self.stop();
        self.history.clear();
    }

    pub fn update_last_exchange(&mut self, answer: &str) {
        self.history.update_last_exchange(answer);
    }

    pub async fn finalize_last_exchange(
        &mut self,
        _tokens_predicted: Option<usize>,
    ) -> Result<(), Box<dyn Error>> {
        // extract the last exchange, trim and tokenize it
        let token_length = if let Some(last_exchange) =
            self.history.get_last_exchange_mut()
        {
            // Strip off trailing whitespaces or newlines from the last exchange
            let trimmed_answer = last_exchange.get_answer().trim().to_string();
            last_exchange.set_answer(trimmed_answer);

            let temp_vec = vec![&*last_exchange];
            let last_prompt_text = ChatHistory::exchanges_to_string(
                self.model.as_ref().ok_or_else(|| "Model not available")?,
                temp_vec,
            )?;

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
            if let Some(last_exchange) = self.history.get_last_exchange_mut() {
                last_exchange.set_token_length(token_length);
            }
        }

        Ok(())
    }

    pub async fn message(
        &mut self,
        tx: mpsc::Sender<Bytes>,
        question: String,
    ) -> Result<(), Box<dyn Error>> {
        let max_token_length = self
            .server
            .get_context_size(&mut self.prompt_instruction)
            .await?;
        let new_exchange = self.initiate_new_exchange(question).await?;
        let n_keep = self.prompt_instruction.get_n_keep();
        let exchanges =
            self.history
                .new_prompt(new_exchange, max_token_length, n_keep);

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancel_tx = Some(cancel_tx); // channel to cancel

        let model = self.model.as_ref().ok_or_else(|| "Model not available")?;

        self.server
            .completion(
                &exchanges,
                model,
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
    ) -> Result<ChatExchange, Box<dyn Error>> {
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

        let last_prompt_text = ChatHistory::exchanges_to_string(
            self.model.as_ref().ok_or_else(|| "Model not available")?,
            temp_vec,
        )?;

        if let Some(token_response) =
            self.server.tokenizer(&last_prompt_text).await?
        {
            new_exchange.set_token_length(token_response.get_tokens().len());
        }
        Ok(new_exchange)
    }

    pub fn process_response(
        &self,
        response: &Bytes,
    ) -> (String, bool, Option<usize>) {
        self.server.process_response(response)
    }

    // used in non-interactive mode
    pub async fn process_prompt(
        &mut self,
        question: String,
        keep_running: Arc<AtomicBool>,
    ) {
        let (tx, rx) = mpsc::channel(32);
        let _ = self.message(tx, question).await;
        self.handle_response(rx, keep_running).await;
    }

    async fn handle_response(
        &self,
        mut rx: mpsc::Receiver<Bytes>,
        keep_running: Arc<AtomicBool>,
    ) {
        while keep_running.load(Ordering::Relaxed) {
            while let Some(response) = rx.recv().await {
                let (response_content, is_final, _) =
                    self.process_response(&response);
                print!("{}", response_content);
                io::stdout().flush().expect("Failed to flush stdout");

                if is_final {
                    break;
                }
            }
        }
    }
}
