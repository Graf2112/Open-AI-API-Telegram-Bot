use crate::storage::Note;

/// API Response structure for Llama model
#[allow(unused)]
#[derive(serde::Deserialize, Debug)]
pub struct Answer {
    /// Unique response identifier
    pub id: String,
    /// Object type identifier
    pub object: String,
    /// Unix timestamp of creation
    pub created: u32,
    /// Model name used for generation
    pub model: String,
    /// Vector of generated choices/responses
    pub choices: Vec<Choice>,
    /// Token usage statistics
    pub usage: Usage,
    /// Model system fingerprint
    pub system_fingerprint: String,
}

/// Structure representing a single response choice
#[allow(unused)]
#[derive(serde::Deserialize, Debug)]
pub struct Choice {
    /// Choice index in the response array
    pub index: u32,
    /// Optional log probabilities
    pub logprobs: Option<String>,
    /// Reason for completion
    pub finish_reason: String,
    /// Generated message content
    pub message: Message,
}

/// Token usage statistics structure
#[allow(unused)]
#[derive(serde::Deserialize, Debug)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Message structure for API communication
#[allow(unused)]
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Message {
    /// Role of the message sender (system/user/assistant)
    pub role: String,
    /// Actual message content
    pub content: String,
    /// Reasoning content (if applicable)
    pub reasoning: Option<String>,
}

impl From<&Note> for Message {
    fn from(note: &Note) -> Self {
        Self {
            role: "user".into(),
            content: note.text.clone(),
            reasoning: None,
        }
    }
}
