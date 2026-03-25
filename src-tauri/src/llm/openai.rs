use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use super::TokenStream;

pub async fn stream(api_key: &str, prompt: &str) -> Result<TokenStream, String> {
    let client = Client::new();
    let body = json!({
        "model": "gpt-4o",
        "stream": true,
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("API key inválida. Verifique nas configurações.".into());
    }
    if !response.status().is_success() {
        return Err(format!("Erro OpenAI API: {}", response.status()));
    }

    let stream = response.bytes_stream().flat_map(|chunk_result| {
        let tokens: Vec<Result<String, String>> = match chunk_result {
            Err(e) => vec![Err(e.to_string())],
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                text.lines()
                    .filter_map(|line| super::extract_sse_data(line))
                    .filter(|data| *data != "[DONE]" && !data.is_empty())
                    .filter_map(|data| serde_json::from_str::<Value>(data).ok())
                    .filter_map(|v| {
                        v["choices"][0]["delta"]["content"]
                            .as_str()
                            .map(|s| Ok(s.to_string()))
                    })
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
    fn test_openai_delta_extraction() {
        let v = json!({"choices": [{"delta": {"content": "Hello"}}]});
        assert_eq!(v["choices"][0]["delta"]["content"].as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_openai_empty_delta_skipped() {
        let v = json!({"choices": [{"delta": {}}]});
        assert!(v["choices"][0]["delta"]["content"].as_str().is_none());
    }
}
