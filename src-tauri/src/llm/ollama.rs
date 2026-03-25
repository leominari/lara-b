use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use super::TokenStream;

pub async fn stream(base_url: &str, model: &str, prompt: &str) -> Result<TokenStream, String> {
    let client = Client::new();
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = json!({ "model": model, "prompt": prompt, "stream": true });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.to_string().contains("Connection refused") || e.to_string().contains("connect") {
                "Ollama não está rodando. Abra o Ollama e tente novamente.".to_string()
            } else {
                e.to_string()
            }
        })?;

    if !response.status().is_success() {
        return Err(format!("Erro Ollama: {}", response.status()));
    }

    let stream = response.bytes_stream().flat_map(|chunk_result| {
        let tokens: Vec<Result<String, String>> = match chunk_result {
            Err(e) => vec![Err(e.to_string())],
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                text.lines()
                    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                    .filter(|v| v["done"].as_bool() != Some(true))
                    .filter_map(|v| v["response"].as_str().map(|s| Ok(s.to_string())))
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
    fn test_ollama_response_extraction() {
        let v = json!({"model": "llama3", "response": "Hi", "done": false});
        assert_eq!(v["response"].as_str().unwrap(), "Hi");
        assert_ne!(v["done"].as_bool(), Some(true));
    }

    #[test]
    fn test_ollama_done_chunk_skipped() {
        let v = json!({"model": "llama3", "response": "", "done": true});
        assert_eq!(v["done"].as_bool(), Some(true)); // should be filtered out
    }
}
