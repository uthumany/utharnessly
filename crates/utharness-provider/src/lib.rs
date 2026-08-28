use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{thread, time::Duration};

#[derive(Clone, Debug)]
pub struct OpenRouter {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    max_retries: usize,
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
        let parsed_url =
            reqwest::Url::parse(&base_url).context("UTHARNESS_PROVIDER_URL is invalid")?;
        let local_http = parsed_url.scheme() == "http"
            && matches!(
                parsed_url.host_str(),
                Some("localhost" | "127.0.0.1" | "::1")
            );
        if parsed_url.scheme() != "https" && !local_http {
            anyhow::bail!(
                "UTHARNESS_PROVIDER_URL must use HTTPS (HTTP is allowed only for localhost)"
            );
        }
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
            max_retries: 2,
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
        for attempt in 0..=self.max_retries {
            let response = self
                .client
                .post(format!("{}/chat/completions", self.base_url))
                .bearer_auth(&self.api_key)
                .header("HTTP-Referer", "https://github.com/uthumany/utharnessly")
                .header("X-Title", "Utharness Agent Terminal")
                .json(&body)
                .send();
            let response = match response {
                Ok(response) => response,
                Err(error)
                    if attempt < self.max_retries && (error.is_timeout() || error.is_connect()) =>
                {
                    thread::sleep(retry_delay(attempt));
                    continue;
                }
                Err(error) => return Err(error).context("OpenRouter request failed"),
            };
            let status = response.status();
            let retryable = status.as_u16() == 429 || status.is_server_error();
            let raw = response
                .text()
                .context("failed to read OpenRouter response")?;
            if retryable && attempt < self.max_retries {
                thread::sleep(retry_delay(attempt));
                continue;
            }
            let payload: ChatResponse = serde_json::from_str(&raw)
                .with_context(|| format!("OpenRouter returned invalid JSON (HTTP {status})"))?;
            if !status.is_success() {
                let message = payload
                    .error
                    .map(|error| error.message)
                    .unwrap_or_else(|| format!("HTTP {status}"));
                anyhow::bail!("OpenRouter request rejected: {message}");
            }
            return payload
                .choices
                .into_iter()
                .next()
                .map(|choice| choice.message.content)
                .filter(|content| !content.trim().is_empty())
                .context("OpenRouter returned no assistant content");
        }
        unreachable!("retry loop always returns")
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

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(250 * (1_u64 << attempt.min(3)))
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

    #[test]
    fn retry_delay_is_bounded_exponential_backoff() {
        assert_eq!(retry_delay(0), Duration::from_millis(250));
        assert_eq!(retry_delay(2), Duration::from_millis(1000));
        assert_eq!(retry_delay(20), Duration::from_millis(2000));
    }
}
