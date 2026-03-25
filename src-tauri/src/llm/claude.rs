use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::pin::Pin;
use super::TokenStream;

pub async fn stream(api_key: &str, prompt: &str) -> Result<TokenStream, String> {
    let client = Client::new();
    let body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 1024,
        "stream": true,
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("API key inválida. Verifique nas configurações.".into());
    }
    if !response.status().is_success() {
        return Err(format!("Erro Claude API: {}", response.status()));
    }

    let stream = response.bytes_stream().flat_map(|chunk_result| {
        let tokens: Vec<Result<String, String>> = match chunk_result {
            Err(e) => vec![Err(e.to_string())],
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                text.lines()
                    .filter_map(|line| super::extract_sse_data(line))
                    .filter(|data| !data.is_empty() && *data != "[DONE]")
                    .filter_map(|data| serde_json::from_str::<Value>(data).ok())
                    .filter(|v| v["type"] == "content_block_delta")
                    .filter_map(|v| v["delta"]["text"].as_str().map(|s| Ok(s.to_string())))
                    .collect()
            }
        };
        futures_util::stream::iter(tokens)
    });

    Ok(Box::pin(stream))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_content_block_delta_extracted() {
        let v = json!({"type": "content_block_delta", "delta": {"type": "text_delta", "text": "Hello"}});
        assert_eq!(v["type"], "content_block_delta");
        assert_eq!(v["delta"]["text"].as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_non_delta_events_ignored() {
        let v = json!({"type": "message_start"});
        assert_ne!(v["type"], "content_block_delta");
    }
}
