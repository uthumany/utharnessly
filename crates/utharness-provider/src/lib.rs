use anyhow::{Context, Result};
use reqwest::{blocking::Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader},
    thread,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    OpenRouter,
    OpenAi,
    Groq,
    Together,
    DeepSeek,
    Fireworks,
    Nvidia,
    Mistral,
    Cerebras,
    Cohere,
    CometApi,
    Cloudflare,
    Ollama,
    OllamaCloud,
    SeekAi,
    Custom,
}

impl ProviderKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openrouter" => Ok(Self::OpenRouter),
            "openai" => Ok(Self::OpenAi),
            "groq" => Ok(Self::Groq),
            "together" => Ok(Self::Together),
            "deepseek" => Ok(Self::DeepSeek),
            "fireworks" => Ok(Self::Fireworks),
            "nvidia" | "nvidia-nim" | "nim" => Ok(Self::Nvidia),
            "mistral" => Ok(Self::Mistral),
            "cerebras" => Ok(Self::Cerebras),
            "cohere" => Ok(Self::Cohere),
            "cometapi" | "comet" => Ok(Self::CometApi),
            "cloudflare" | "cf" | "workers-ai" => Ok(Self::Cloudflare),
            "ollama" | "local" => Ok(Self::Ollama),
            "ollama-cloud" | "ollama_cloud" | "ollama-api" => Ok(Self::OllamaCloud),
            "seekai" | "seek" => Ok(Self::SeekAi),
            "custom" | "openai-compatible" => Ok(Self::Custom),
            other => anyhow::bail!("unsupported provider '{other}'"),
        }
    }
    pub fn id(&self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::OpenAi => "openai",
            Self::Groq => "groq",
            Self::Together => "together",
            Self::DeepSeek => "deepseek",
            Self::Fireworks => "fireworks",
            Self::Nvidia => "nvidia",
            Self::Mistral => "mistral",
            Self::Cerebras => "cerebras",
            Self::Cohere => "cohere",
            Self::CometApi => "cometapi",
            Self::Cloudflare => "cloudflare",
            Self::Ollama => "ollama",
            Self::OllamaCloud => "ollama-cloud",
            Self::SeekAi => "seekai",
            Self::Custom => "custom",
        }
    }
    fn defaults(&self) -> (&'static str, &'static str, Option<&'static str>) {
        match self {
            Self::OpenRouter => (
                "https://openrouter.ai/api/v1",
                "openrouter/free",
                Some("OPENROUTER_API_KEY"),
            ),
            Self::OpenAi => (
                "https://api.openai.com/v1",
                "gpt-4o-mini",
                Some("OPENAI_API_KEY"),
            ),
            Self::Groq => (
                "https://api.groq.com/openai/v1",
                "groq/compound-mini",
                Some("GROQ_API_KEY"),
            ),
            Self::Together => (
                "https://api.together.xyz/v1",
                "meta-llama/Llama-3.3-70B-Instruct-Turbo",
                Some("TOGETHER_API_KEY"),
            ),
            Self::DeepSeek => (
                "https://api.deepseek.com/v1",
                "deepseek-chat",
                Some("DEEPSEEK_API_KEY"),
            ),
            Self::Fireworks => (
                "https://api.fireworks.ai/inference/v1",
                "accounts/fireworks/models/llama-v3p3-70b-instruct",
                Some("FIREWORKS_API_KEY"),
            ),
            Self::Nvidia => (
                "https://integrate.api.nvidia.com/v1",
                "nvidia/nemotron-3-super-120b-a12b",
                Some("NVIDIA_API_KEY"),
            ),
            Self::Mistral => (
                "https://api.mistral.ai/v1",
                "mistral-small-latest",
                Some("MISTRAL_API_KEY"),
            ),
            Self::Cerebras => (
                "https://api.cerebras.ai/v1",
                "qwen-3.8-27b",
                Some("CEREBRAS_API_KEY"),
            ),
            Self::Cohere => (
                "https://api.cohere.com/compatibility/v1",
                "command-a-03-2025",
                Some("COHERE_API_KEY"),
            ),
            Self::CometApi => (
                "https://api.cometapi.com/v1",
                "gpt-4o-mini",
                Some("COMETAPI_API_KEY"),
            ),
            // Workers AI has no GET /models endpoint; the account-scoped
            // OpenAI-compatible base URL is built from CLOUDFLARE_ACCOUNT_ID
            // in `cloudflare_default_base_url`. This placeholder is only
            // displayed when the account id is missing.
            Self::Cloudflare => (
                "https://api.cloudflare.com/client/v4/accounts/{ACCOUNT_ID}/ai/v1",
                "@cf/qwen/qwen3.8-27b",
                Some("CLOUDFLARE_API_TOKEN"),
            ),
            Self::Ollama => ("http://127.0.0.1:11434/v1", "qwen2.5-coder:7b", None),
            // Ollama Cloud serves GET /v1/models (OpenAI shape) but chat
            // only via the native POST /api/chat protocol (OpenAI chat
            // completions return 405). complete()/complete_streaming()
            // branch on this kind accordingly.
            Self::OllamaCloud => (
                "https://api.ollama.com",
                "nemotron-3-ultra",
                Some("OLLAMA_API_KEY"),
            ),
            // Verified live: the api. subdomain is dead (DNS); the apex
            // host serves OpenAI-compatible /v1/models + SSE chat.
            // Default mimo-v2.5 streams with content; deepseek-v4-flash
            // stalls server-side and hy3 returns reasoning-only payloads.
            Self::SeekAi => ("https://seekai.cc/v1", "mimo-v2.5", Some("SEEKAI_API_KEY")),
            Self::Custom => (
                "http://127.0.0.1:8000/v1",
                "default",
                Some("UTHARNESS_API_KEY"),
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Gateway {
    client: Client,
    kind: ProviderKind,
    api_key: Option<String>,
    base_url: String,
    model: String,
    max_retries: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStatus {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub credential_source: Option<String>,
    pub configured: bool,
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
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelRecord>,
}
#[derive(Debug, Deserialize)]
struct ModelRecord {
    id: String,
}
#[derive(Debug, Deserialize)]
struct StreamPayload {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    error: Option<ApiError>,
}
#[derive(Debug, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
}
#[derive(Debug, Default, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    #[serde(default)]
    message: Option<OllamaChatMessage>,
    #[serde(default)]
    done: bool,
}
#[derive(Debug, Deserialize)]
struct OllamaChatMessage {
    #[serde(default)]
    content: String,
}

impl Gateway {
    pub fn from_environment() -> Result<Self> {
        let provider = if let Ok(value) = std::env::var("UTHARNESS_PROVIDER") {
            ProviderKind::parse(&value)?
        } else if environment_has_value("OPENROUTER_API_KEY") {
            ProviderKind::OpenRouter
        } else if environment_has_value("OPENAI_API_KEY") {
            ProviderKind::OpenAi
        } else if environment_has_value("GROQ_API_KEY") {
            ProviderKind::Groq
        } else if environment_has_value("TOGETHER_API_KEY") {
            ProviderKind::Together
        } else if environment_has_value("DEEPSEEK_API_KEY") {
            ProviderKind::DeepSeek
        } else if environment_has_value("FIREWORKS_API_KEY") {
            ProviderKind::Fireworks
        } else if environment_has_value("NVIDIA_API_KEY") {
            ProviderKind::Nvidia
        } else if environment_has_value("MISTRAL_API_KEY") {
            ProviderKind::Mistral
        } else if environment_has_value("CEREBRAS_API_KEY") {
            ProviderKind::Cerebras
        } else if environment_has_value("COHERE_API_KEY") {
            ProviderKind::Cohere
        } else if environment_has_value("COMETAPI_API_KEY") {
            ProviderKind::CometApi
        } else if environment_has_value("CLOUDFLARE_API_TOKEN") {
            ProviderKind::Cloudflare
        } else if environment_has_value("OLLAMA_API_KEY") {
            ProviderKind::OllamaCloud
        } else if environment_has_value("SEEKAI_API_KEY") {
            ProviderKind::SeekAi
        } else {
            anyhow::bail!("no AI gateway is configured; set UTHARNESS_PROVIDER or a supported provider API key")
        };
        Self::new_from_environment(provider)
    }
    pub fn new_from_environment(kind: ProviderKind) -> Result<Self> {
        let (default_url, default_model, key_variable) = kind.defaults();
        let base_url = match std::env::var("UTHARNESS_PROVIDER_URL") {
            Ok(url) if !url.trim().is_empty() => url,
            _ if kind == ProviderKind::Cloudflare => cloudflare_default_base_url()?,
            _ => default_url.into(),
        };
        validate_base_url(&base_url)?;
        let source = std::env::var("UTHARNESS_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(|_| "UTHARNESS_API_KEY")
            .or_else(|| {
                key_variable.and_then(|name| {
                    std::env::var(name)
                        .ok()
                        .filter(|v| !v.trim().is_empty())
                        .map(|_| name)
                })
            });
        let api_key = source.and_then(|name| std::env::var(name).ok());
        if kind != ProviderKind::Ollama && api_key.is_none() {
            anyhow::bail!(
                "{} is not configured; set {} or UTHARNESS_API_KEY",
                kind.id(),
                key_variable.unwrap_or("UTHARNESS_API_KEY")
            );
        }
        let client = Client::builder()
            // Default reqwest UA ("reqwest/x.y") is tar-pitted by some
            // provider WAFs (observed: request hangs with zero bytes
            // returned). A product UA is standard practice everywhere.
            .user_agent(concat!("utharness/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .context("failed to build provider HTTP client")?;
        Ok(Self {
            client,
            kind,
            api_key,
            base_url: base_url.trim_end_matches('/').into(),
            model: std::env::var("UTHARNESS_MODEL").unwrap_or_else(|_| default_model.into()),
            max_retries: 2,
        })
    }
    pub fn status_from_environment(kind: ProviderKind) -> ProviderStatus {
        let (default_url, default_model, key_variable) = kind.defaults();
        let source = if std::env::var("UTHARNESS_API_KEY").is_ok_and(|v| !v.trim().is_empty()) {
            Some("UTHARNESS_API_KEY".into())
        } else {
            key_variable
                .filter(|name| std::env::var(name).is_ok_and(|v| !v.trim().is_empty()))
                .map(str::to_string)
        };
        ProviderStatus {
            provider: kind.id().into(),
            model: std::env::var("UTHARNESS_MODEL").unwrap_or_else(|_| default_model.into()),
            base_url: match std::env::var("UTHARNESS_PROVIDER_URL") {
                Ok(url) if !url.trim().is_empty() => url,
                _ if kind == ProviderKind::Cloudflare => {
                    cloudflare_default_base_url().unwrap_or_else(|_| default_url.into())
                }
                _ => default_url.into(),
            },
            configured: kind == ProviderKind::Ollama || source.is_some(),
            credential_source: source,
        }
    }
    pub fn provider(&self) -> &str {
        self.kind.id()
    }
    pub fn model(&self) -> &str {
        &self.model
    }
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
    fn models_url(&self) -> String {
        if self.kind == ProviderKind::OllamaCloud {
            format!("{}/v1/models", self.base_url)
        } else {
            format!("{}/models", self.base_url)
        }
    }
    pub fn health_check(&self) -> Result<StatusCode> {
        // Workers AI exposes no GET /models endpoint; validate the token
        // against the account resource instead.
        if self.kind == ProviderKind::Cloudflare {
            let account = cloudflare_account_id()?;
            let response = self
                .authorize(self.client.get(format!(
                    "https://api.cloudflare.com/client/v4/accounts/{account}"
                )))
                .send()
                .context("provider health request failed")?;
            let status = response.status();
            if !status.is_success() {
                anyhow::bail!("{} health check failed with HTTP {status}", self.kind.id());
            }
            return Ok(status);
        }
        let response = self
            .authorize(self.client.get(self.models_url()))
            .send()
            .context("provider health request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("{} health check failed with HTTP {status}", self.kind.id());
        }
        Ok(status)
    }
    pub fn models(&self) -> Result<Vec<String>> {
        // Workers AI has no list-models endpoint; advertise the models
        // verified to serve OpenAI-compatible chat on the free plan.
        if self.kind == ProviderKind::Cloudflare {
            return Ok(vec![
                "@cf/qwen/qwen3.8-27b".into(),
                "@cf/google/gemma-4-26b-a4b-it".into(),
            ]);
        }
        let response = self
            .authorize(self.client.get(self.models_url()))
            .send()
            .context("provider model-list request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("{} model list failed with HTTP {status}", self.kind.id());
        }
        let payload: ModelsResponse = response
            .json()
            .context("provider returned an invalid model list")?;
        let mut models = payload
            .data
            .into_iter()
            .map(|model| model.id)
            .collect::<Vec<_>>();
        models.sort();
        models.dedup();
        Ok(models)
    }
    pub fn validate_model(&self) -> Result<()> {
        // Workers AI cannot enumerate models; accept the advertised list
        // plus any well-formed Workers AI model id.
        if self.kind == ProviderKind::Cloudflare
            && (self.model.starts_with("@cf/") || self.model.starts_with("@hf/"))
        {
            return Ok(());
        }
        let models = self.models()?;
        if models.is_empty() || models.iter().any(|model| model == &self.model) {
            return Ok(());
        }
        anyhow::bail!(
            "model '{}' is not exposed by {}; choose one returned by `utharness models list`",
            self.model,
            self.kind.id()
        )
    }
    pub fn complete(&self, messages: &[ChatMessage], temperature: f32) -> Result<String> {
        // Ollama Cloud rejects OpenAI chat completions (405); use the
        // native /api/chat protocol instead.
        if self.kind == ProviderKind::OllamaCloud {
            return self.ollama_complete(messages, temperature, 1200);
        }
        let body = serde_json::json!({"model": self.model, "messages": messages, "temperature": temperature, "max_tokens": 1200});
        for attempt in 0..=self.max_retries {
            let response = self
                .authorize(
                    self.client
                        .post(format!("{}/chat/completions", self.base_url)),
                )
                .json(&body)
                .send();
            let response = match response {
                Ok(v) => v,
                Err(error)
                    if attempt < self.max_retries && (error.is_timeout() || error.is_connect()) =>
                {
                    thread::sleep(retry_delay(attempt));
                    continue;
                }
                Err(error) => return Err(error).context("provider request failed"),
            };
            let status = response.status();
            let retryable = status.as_u16() == 429 || status.is_server_error();
            let raw = response
                .text()
                .context("failed to read provider response")?;
            if retryable && attempt < self.max_retries {
                thread::sleep(retry_delay(attempt));
                continue;
            }
            let payload: ChatResponse = serde_json::from_str(&raw)
                .with_context(|| format!("provider returned invalid JSON (HTTP {status})"))?;
            if !status.is_success() {
                anyhow::bail!(
                    "provider request rejected: {}",
                    payload
                        .error
                        .map(|e| e.message)
                        .unwrap_or_else(|| format!("HTTP {status}"))
                );
            }
            return payload
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .filter(|v| !v.trim().is_empty())
                .context("provider returned no assistant content");
        }
        unreachable!("retry loop always returns")
    }
    pub fn complete_streaming<F>(&self, messages: &[ChatMessage], mut on_delta: F) -> Result<String>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let body = serde_json::json!({"model": self.model, "messages": messages, "temperature": 0.2, "max_tokens": 1600, "stream": true});
        if self.kind == ProviderKind::OllamaCloud {
            return self.ollama_complete_streaming(messages, on_delta);
        }
        let response = self
            .authorize(
                self.client
                    .post(format!("{}/chat/completions", self.base_url)),
            )
            .json(&body)
            .send()
            .context("streaming provider request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("streaming provider request rejected (HTTP {status})");
        }
        let mut complete = String::new();
        for line in BufReader::new(response).lines() {
            let line = line.context("failed reading provider stream")?;
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                break;
            }
            let payload: StreamPayload =
                serde_json::from_str(data).context("provider stream contained invalid JSON")?;
            if let Some(error) = payload.error {
                anyhow::bail!("provider stream failed: {}", error.message);
            }
            for choice in payload.choices {
                if let Some(delta) = choice.delta.content.filter(|v| !v.is_empty()) {
                    on_delta(&delta)?;
                    complete.push_str(&delta);
                }
            }
        }
        if complete.trim().is_empty() {
            anyhow::bail!("provider stream completed without assistant content");
        }
        Ok(complete)
    }
    fn ollama_complete(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {"temperature": temperature, "num_predict": max_tokens}
        });
        for attempt in 0..=self.max_retries {
            let response = self
                .authorize(self.client.post(format!("{}/api/chat", self.base_url)))
                .json(&body)
                .send();
            let response = match response {
                Ok(v) => v,
                Err(error)
                    if attempt < self.max_retries && (error.is_timeout() || error.is_connect()) =>
                {
                    thread::sleep(retry_delay(attempt));
                    continue;
                }
                Err(error) => return Err(error).context("ollama-cloud request failed"),
            };
            let status = response.status();
            let raw = response
                .text()
                .context("failed to read ollama-cloud response")?;
            if !status.is_success() {
                if attempt < self.max_retries
                    && (status.as_u16() == 429 || status.is_server_error())
                {
                    thread::sleep(retry_delay(attempt));
                    continue;
                }
                anyhow::bail!(
                    "ollama-cloud request rejected (HTTP {status}): {}",
                    raw.chars().take(200).collect::<String>()
                );
            }
            let payload: OllamaChatResponse = serde_json::from_str(&raw)
                .with_context(|| "ollama-cloud returned invalid JSON".to_string())?;
            return payload
                .message
                .map(|m| m.content)
                .filter(|v| !v.trim().is_empty())
                .context("ollama-cloud returned no assistant content");
        }
        unreachable!("retry loop always returns")
    }
    fn ollama_complete_streaming<F>(
        &self,
        messages: &[ChatMessage],
        mut on_delta: F,
    ) -> Result<String>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "options": {"temperature": 0.2, "num_predict": 1600}
        });
        let response = self
            .authorize(self.client.post(format!("{}/api/chat", self.base_url)))
            .json(&body)
            .send()
            .context("streaming ollama-cloud request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("streaming ollama-cloud request rejected (HTTP {status})");
        }
        let mut complete = String::new();
        for line in BufReader::new(response).lines() {
            let line = line.context("failed reading ollama-cloud stream")?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line.contains("\"error\"") {
                anyhow::bail!(
                    "ollama-cloud stream failed: {}",
                    line.chars().take(200).collect::<String>()
                );
            }
            let payload: OllamaChatResponse =
                serde_json::from_str(line).context("ollama-cloud stream contained invalid JSON")?;
            if payload.done {
                break;
            }
            if let Some(delta) = payload.message.map(|m| m.content).filter(|v| !v.is_empty()) {
                on_delta(&delta)?;
                complete.push_str(&delta);
            }
        }
        if complete.trim().is_empty() {
            anyhow::bail!("ollama-cloud stream completed without assistant content");
        }
        Ok(complete)
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
    fn authorize(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        let request = if let Some(key) = &self.api_key {
            request.bearer_auth(key)
        } else {
            request
        };
        if self.kind == ProviderKind::OpenRouter {
            request
                .header("HTTP-Referer", "https://github.com/uthumany/utharnessly")
                .header("X-Title", "Utharness Agent Terminal")
        } else {
            request
        }
    }
}

pub type OpenRouter = Gateway;
pub fn supported_providers() -> Vec<ProviderStatus> {
    [
        ProviderKind::OpenRouter,
        ProviderKind::OpenAi,
        ProviderKind::Groq,
        ProviderKind::Together,
        ProviderKind::DeepSeek,
        ProviderKind::Fireworks,
        ProviderKind::Nvidia,
        ProviderKind::Mistral,
        ProviderKind::Cerebras,
        ProviderKind::Cohere,
        ProviderKind::CometApi,
        ProviderKind::Cloudflare,
        ProviderKind::Ollama,
        ProviderKind::OllamaCloud,
        ProviderKind::SeekAi,
        ProviderKind::Custom,
    ]
    .into_iter()
    .map(Gateway::status_from_environment)
    .collect()
}

pub fn has_provider_configuration() -> bool {
    std::env::var("UTHARNESS_PROVIDER").is_ok_and(|value| !value.trim().is_empty())
        || [
            "UTHARNESS_API_KEY",
            "OPENROUTER_API_KEY",
            "OPENAI_API_KEY",
            "GROQ_API_KEY",
            "TOGETHER_API_KEY",
            "DEEPSEEK_API_KEY",
            "FIREWORKS_API_KEY",
            "NVIDIA_API_KEY",
            "MISTRAL_API_KEY",
            "CEREBRAS_API_KEY",
            "COHERE_API_KEY",
            "COMETAPI_API_KEY",
            "CLOUDFLARE_API_TOKEN",
            "OLLAMA_API_KEY",
            "SEEKAI_API_KEY",
        ]
        .into_iter()
        .any(environment_has_value)
}

fn environment_has_value(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| !value.trim().is_empty())
}
fn cloudflare_account_id() -> Result<String> {
    std::env::var("CLOUDFLARE_ACCOUNT_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .context(
            "cloudflare is not configured; set CLOUDFLARE_ACCOUNT_ID to your Workers AI account id",
        )
}
fn cloudflare_default_base_url() -> Result<String> {
    Ok(format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/ai/v1",
        cloudflare_account_id()?
    ))
}
fn validate_base_url(value: &str) -> Result<()> {
    let url = Url::parse(value).context("UTHARNESS_PROVIDER_URL is invalid")?;
    let local_http =
        url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if url.scheme() != "https" && !local_http {
        anyhow::bail!("provider URL must use HTTPS; HTTP is allowed only for loopback hosts");
    }
    Ok(())
}
fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(250 * (1_u64 << attempt.min(3)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    #[test]
    fn chat_messages_serialize_for_openai_compatible_api() {
        let value = serde_json::to_value(ChatMessage {
            role: "user".into(),
            content: "hello".into(),
        })
        .unwrap();
        assert_eq!(value["role"], "user");
    }
    #[test]
    fn provider_names_are_explicit() {
        assert_eq!(ProviderKind::parse("ollama").unwrap(), ProviderKind::Ollama);
        assert_eq!(ProviderKind::parse("nim").unwrap(), ProviderKind::Nvidia);
        assert_eq!(
            ProviderKind::parse("mistral").unwrap(),
            ProviderKind::Mistral
        );
        assert_eq!(
            ProviderKind::parse("cerebras").unwrap(),
            ProviderKind::Cerebras
        );
        assert_eq!(ProviderKind::parse("cohere").unwrap(), ProviderKind::Cohere);
        assert_eq!(
            ProviderKind::parse("cometapi").unwrap(),
            ProviderKind::CometApi
        );
        assert_eq!(
            ProviderKind::parse("cloudflare").unwrap(),
            ProviderKind::Cloudflare
        );
        assert_eq!(ProviderKind::parse("cf").unwrap(), ProviderKind::Cloudflare);
        assert_eq!(
            ProviderKind::parse("ollama-cloud").unwrap(),
            ProviderKind::OllamaCloud
        );
        assert_eq!(ProviderKind::parse("seekai").unwrap(), ProviderKind::SeekAi);
        assert!(ProviderKind::parse("unknown").is_err());
    }
    #[test]
    fn retry_delay_is_bounded_exponential_backoff() {
        assert_eq!(retry_delay(0), Duration::from_millis(250));
        assert_eq!(retry_delay(20), Duration::from_millis(2000));
    }
    #[test]
    fn rejects_non_loopback_plain_http() {
        assert!(validate_base_url("http://example.com/v1").is_err());
        assert!(validate_base_url("http://127.0.0.1:8000/v1").is_ok());
    }
    #[test]
    fn streams_openai_compatible_deltas_in_order() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0_u8; 8192];
            let read = socket.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..read]).contains("POST /v1/chat/completions"));
            let body = "data: {\"choices\":[{\"delta\":{\"content\":\"real \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"time\"}}]}\n\ndata: [DONE]\n\n";
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let gateway = Gateway {
            client: Client::builder().build().unwrap(),
            kind: ProviderKind::Custom,
            api_key: Some("test-only".into()),
            base_url: format!("http://{address}/v1"),
            model: "fixture".into(),
            max_retries: 0,
        };
        let mut deltas = Vec::new();
        let complete = gateway
            .complete_streaming(
                &[ChatMessage {
                    role: "user".into(),
                    content: "hello".into(),
                }],
                |delta| {
                    deltas.push(delta.to_string());
                    Ok(())
                },
            )
            .unwrap();
        server.join().unwrap();
        assert_eq!(deltas, ["real ", "time"]);
        assert_eq!(complete, "real time");
    }
    #[test]
    fn ollama_cloud_chat_uses_native_api_protocol() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for first in [true, false] {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0_u8; 8192];
                let read = socket.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..read]);
                assert!(request.contains("POST /api/chat"));
                assert!(request.contains("Bearer test-only"));
                let body = if first {
                    "{\"model\":\"fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"cloud hi\"},\"done\":true}".to_string()
                } else {
                    "{\"model\":\"fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"cloud \"},\"done\":false}\n{\"model\":\"fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"hi\"},\"done\":false}\n{\"model\":\"fixture\",\"done\":true}\n".to_string()
                };
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        let gateway = Gateway {
            client: Client::builder().build().unwrap(),
            kind: ProviderKind::OllamaCloud,
            api_key: Some("test-only".into()),
            base_url: format!("http://{address}"),
            model: "fixture".into(),
            max_retries: 0,
        };
        let messages = [ChatMessage {
            role: "user".into(),
            content: "hello".into(),
        }];
        assert_eq!(
            gateway.ollama_complete(&messages, 0.2, 50).unwrap(),
            "cloud hi"
        );
        let mut deltas = Vec::new();
        let complete = gateway
            .ollama_complete_streaming(&messages, |delta| {
                deltas.push(delta.to_string());
                Ok(())
            })
            .unwrap();
        server.join().unwrap();
        assert_eq!(deltas, ["cloud ", "hi"]);
        assert_eq!(complete, "cloud hi");
    }
}
