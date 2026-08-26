use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct OpenRouter {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

impl OpenRouter {
    pub fn from_environment() -> Result<Self> {
        let api_key =
            std::env::var("OPENROUTER_API_KEY").context("OPENROUTER_API_KEY is not set")?;
        if api_key.trim().is_empty() {
            anyhow::bail!("OPENROUTER_API_KEY is empty");
        }
        let base_url = std::env::var("UTHARNESS_PROVIDER_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".into());
        let model = std::env::var("UTHARNESS_MODEL").unwrap_or_else(|_| "openrouter/free".into());
        let client = Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .context("failed to build provider HTTP client")?;
        Ok(Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').into(),
            model,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn complete(&self, messages: &[ChatMessage], temperature: f32) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": 1200,
        });
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/uthumany/utharnessly")
            .header("X-Title", "Utharness Agent Terminal")
            .json(&body)
            .send()
            .context("OpenRouter request failed")?;
        let status = response.status();
        let payload: ChatResponse = response
            .json()
            .context("OpenRouter returned invalid JSON")?;
        if !status.is_success() {
            let message = payload
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| format!("HTTP {status}"));
            anyhow::bail!("OpenRouter request rejected: {message}");
        }
        payload
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .context("OpenRouter returned no assistant content")
    }

    pub fn complete_json<T: for<'de> Deserialize<'de>>(
        &self,
        messages: &[ChatMessage],
    ) -> Result<T> {
        let raw = self.complete(messages, 0.1)?;
        let cleaned = raw
            .trim()
            .strip_prefix("```json")
            .unwrap_or(raw.trim())
            .strip_suffix("```")
            .unwrap_or_else(|| raw.trim())
            .trim();
        serde_json::from_str(cleaned).context("provider response was not valid requested JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_messages_serialize_for_openai_compatible_api() {
        let message = ChatMessage {
            role: "user".into(),
            content: "hello".into(),
        };
        let value = serde_json::to_value(message).unwrap();
        assert_eq!(value["role"], "user");
        assert_eq!(value["content"], "hello");
    }
}
