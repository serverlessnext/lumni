#[derive(Debug, Default, Clone)]
pub struct ContentChunk {
    pub content: String,
}
#[derive(Debug, Default, Clone)]
pub struct CompletionStats {
    pub last_token_received_at: u64, // time (epoch/ms) of last token received by client
    pub total_duration: Option<usize>, // duration in milliseconds (server provided)
    pub tokens_in_prompt: Option<usize>, // numbers of tokens in prompt
    pub tokens_evaluated: Option<usize>, // numbers of tokens evaluated
    pub tokens_predicted: Option<usize>, // numbers of tokens predicted
}

impl CompletionStats {
    pub fn merge(&mut self, other: &CompletionStats) {
        // Update last_token_received_at to the latest time
        self.last_token_received_at = self
            .last_token_received_at
            .max(other.last_token_received_at);

        // For optional fields, prefer the other stats if available
        if let Some(duration) = other.total_duration {
            self.total_duration = Some(duration);
        }
        if let Some(tokens) = other.tokens_in_prompt {
            self.tokens_in_prompt = Some(tokens);
        }
        if let Some(tokens) = other.tokens_evaluated {
            self.tokens_evaluated = Some(tokens);
        }
        if let Some(tokens) = other.tokens_predicted {
            self.tokens_predicted = Some(tokens);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CompletionResponse {
    pub chunks: Vec<ContentChunk>,
    pub is_final: bool,
    pub stats: Option<CompletionStats>,
}

impl CompletionResponse {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            is_final: false,
            stats: None,
        }
    }

    pub fn new_content(content: String) -> Self {
        let mut response = Self::new();
        response.append(content);
        response
    }

    pub fn new_final(content: String, stats: Option<CompletionStats>) -> Self {
        let mut response = Self::new_content(content);
        response.set_final();
        response.stats = stats;
        response
    }

    pub fn append(&mut self, content: String) {
        self.chunks.push(ContentChunk { content });
    }

    pub fn set_final(&mut self) {
        self.is_final = true;
    }

    pub fn get_content(&self) -> String {
        self.chunks
            .iter()
            .map(|chunk| chunk.content.clone())
            .collect()
    }

    pub fn get_tokens_predicted(&self) -> Option<usize> {
        self.stats.as_ref().and_then(|s| s.tokens_predicted)
    }
}
