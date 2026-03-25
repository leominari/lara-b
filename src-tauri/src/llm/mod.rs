pub mod claude;
pub mod openai;
pub mod ollama;

use std::pin::Pin;
use futures_util::Stream;

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String, String>> + Send>>;

pub enum LlmConfig {
    Claude { api_key: String },
    OpenAi { api_key: String },
    Ollama { base_url: String, model: String },
}

pub async fn stream_completion(config: LlmConfig, prompt: String) -> Result<TokenStream, String> {
    match config {
        LlmConfig::Claude { api_key } => claude::stream(&api_key, &prompt).await,
        LlmConfig::OpenAi { api_key } => openai::stream(&api_key, &prompt).await,
        LlmConfig::Ollama { base_url, model } => ollama::stream(&base_url, &model, &prompt).await,
    }
}

/// Parse a single SSE data line into text token (shared by Claude + OpenAI tests)
pub fn extract_sse_data(line: &str) -> Option<&str> {
    line.strip_prefix("data: ").map(|s| s.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_data_extraction() {
        assert_eq!(extract_sse_data("data: hello"), Some("hello"));
        assert_eq!(extract_sse_data("event: ping"), None);
        assert_eq!(extract_sse_data("data: "), Some(""));
    }
}
