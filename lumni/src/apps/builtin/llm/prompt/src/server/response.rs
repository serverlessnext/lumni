#[derive(Debug, Default, Clone)]
pub struct ContentChunk {
    pub content: String,
    pub tokens_predicted: Option<usize>,
}

#[derive(Debug, Default, Clone)]
pub struct StreamResponse {
    pub chunks: Vec<ContentChunk>,
    pub is_final: bool,
}

impl StreamResponse {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            is_final: false,
        }
    }

    pub fn new_with_content(
        content: String,
        tokens_predicted: Option<usize>,
        is_final: bool,
    ) -> Self {
        let mut response = Self::new();
        response.append(content, tokens_predicted);
        if is_final {
            response.set_final();
        }
        response
    }

    pub fn append(&mut self, content: String, tokens_predicted: Option<usize>) {
        self.chunks.push(ContentChunk {
            content,
            tokens_predicted,
        });
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

    pub fn get_total_tokens(&self) -> Option<usize> {
        self.chunks.iter().fold(Some(0), |acc, chunk| {
            match (acc, chunk.tokens_predicted) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            }
        })
    }
}
