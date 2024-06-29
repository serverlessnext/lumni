use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockRequestPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_request_fields: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_response_field_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrail_config: Option<GuardrailConfig>,
    pub inference_config: InferenceConfig,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<Document>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use: Option<ToolUse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_content: Option<GuardContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
    pub format: String,
    pub source: ImageSource,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSource {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub format: String,
    pub name: String,
    pub source: DocumentSource,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentSource {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolUse {
    pub tool_use_id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: Vec<ToolContent>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<Document>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardContent {
    pub text: TextContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardrailConfig {
    pub guardrail_identifier: String,
    pub guardrail_version: String,
    pub stream_processing_mode: String,
    pub trace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    pub temperature: f32,
    pub top_p: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMessage {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_content: Option<GuardContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    pub tool_choice: ToolChoice,
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChoice {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolChoiceSpecific>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChoiceSpecific {
    pub name: String,
}
