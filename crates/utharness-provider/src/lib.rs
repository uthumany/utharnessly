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
    Xai,
    Nebius,
    Moonshot,
    MiniMax,
    Upstage,
    Hyperbolic,
    Zhipu,
    DashScope,
    Volcengine,
    StepFun,
    SiliconFlow,
    SambaNova,
    Novita,
    HuggingFace,
    Vercel,
    Chutes,
    OvhCloud,
    Mimo,
    Custom,
}

/// One row per provider. Adding a provider is adding a row here:
/// `parse`, `id`, `defaults`, auto-detect, `supported_providers`,
/// `has_provider_configuration`, and the CLI key map all derive from it.
struct ProviderDef {
    kind: ProviderKind,
    id: &'static str,
    aliases: &'static [&'static str],
    default_url: &'static str,
    default_model: &'static str,
    key_var: Option<&'static str>,
    /// Extra non-secret env the provider needs (e.g. an account id).
    extra_env: Option<&'static str>,
    /// Whether a set key auto-selects this provider when
    /// UTHARNESS_PROVIDER is unset. False for keyless local Ollama and
    /// for `custom`, whose key is an explicit override, not a selector.
    detect: bool,
}

const PROVIDERS: &[ProviderDef] = &[
    ProviderDef {
        kind: ProviderKind::OpenRouter,
        id: "openrouter",
        aliases: &[],
        default_url: "https://openrouter.ai/api/v1",
        default_model: "openrouter/free",
        key_var: Some("OPENROUTER_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::OpenAi,
        id: "openai",
        aliases: &[],
        default_url: "https://api.openai.com/v1",
        default_model: "gpt-4o-mini",
        key_var: Some("OPENAI_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Groq,
        id: "groq",
        aliases: &[],
        default_url: "https://api.groq.com/openai/v1",
        default_model: "groq/compound-mini",
        key_var: Some("GROQ_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Together,
        id: "together",
        aliases: &[],
        default_url: "https://api.together.xyz/v1",
        default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
        key_var: Some("TOGETHER_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::DeepSeek,
        id: "deepseek",
        aliases: &[],
        default_url: "https://api.deepseek.com/v1",
        default_model: "deepseek-chat",
        key_var: Some("DEEPSEEK_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Fireworks,
        id: "fireworks",
        aliases: &[],
        default_url: "https://api.fireworks.ai/inference/v1",
        default_model: "accounts/fireworks/models/llama-v3p3-70b-instruct",
        key_var: Some("FIREWORKS_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Nvidia,
        id: "nvidia",
        aliases: &["nvidia-nim", "nim"],
        default_url: "https://integrate.api.nvidia.com/v1",
        default_model: "nvidia/nemotron-3-super-120b-a12b",
        key_var: Some("NVIDIA_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Mistral,
        id: "mistral",
        aliases: &[],
        default_url: "https://api.mistral.ai/v1",
        default_model: "mistral-small-latest",
        key_var: Some("MISTRAL_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Cerebras,
        id: "cerebras",
        aliases: &[],
        default_url: "https://api.cerebras.ai/v1",
        default_model: "qwen-3.8-27b",
        key_var: Some("CEREBRAS_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Cohere,
        id: "cohere",
        aliases: &[],
        default_url: "https://api.cohere.com/compatibility/v1",
        default_model: "command-a-03-2025",
        key_var: Some("COHERE_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::CometApi,
        id: "cometapi",
        aliases: &["comet"],
        default_url: "https://api.cometapi.com/v1",
        default_model: "gpt-4o-mini",
        key_var: Some("COMETAPI_API_KEY"),
        extra_env: None,
        detect: true,
    },
    // Workers AI has no GET /models endpoint; the account-scoped
    // OpenAI-compatible base URL is built from CLOUDFLARE_ACCOUNT_ID
    // in `cloudflare_default_base_url`. This placeholder is only
    // displayed when the account id is missing.
    ProviderDef {
        kind: ProviderKind::Cloudflare,
        id: "cloudflare",
        aliases: &["cf", "workers-ai"],
        default_url: "https://api.cloudflare.com/client/v4/accounts/{ACCOUNT_ID}/ai/v1",
        default_model: "@cf/qwen/qwen3.8-27b",
        key_var: Some("CLOUDFLARE_API_TOKEN"),
        extra_env: Some("CLOUDFLARE_ACCOUNT_ID"),
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Ollama,
        id: "ollama",
        aliases: &["local"],
        default_url: "http://127.0.0.1:11434/v1",
        default_model: "qwen2.5-coder:7b",
        key_var: None,
        extra_env: None,
        detect: false,
    },
    // Ollama Cloud serves GET /v1/models (OpenAI shape) but chat
    // only via the native POST /api/chat protocol (OpenAI chat
    // completions return 405). complete()/complete_streaming()
    // branch on this kind accordingly.
    ProviderDef {
        kind: ProviderKind::OllamaCloud,
        id: "ollama-cloud",
        aliases: &["ollama_cloud", "ollama-api"],
        default_url: "https://api.ollama.com",
        default_model: "nemotron-3-ultra",
        key_var: Some("OLLAMA_API_KEY"),
        extra_env: None,
        detect: true,
    },
    // Verified live: the api. subdomain is dead (DNS); the apex
    // host serves OpenAI-compatible /v1/models + SSE chat.
    // Default mimo-v2.5 streams with content; deepseek-v4-flash
    // stalls server-side and hy3 returns reasoning-only payloads.
    ProviderDef {
        kind: ProviderKind::SeekAi,
        id: "seekai",
        aliases: &["seek"],
        default_url: "https://seekai.cc/v1",
        default_model: "mimo-v2.5",
        key_var: Some("SEEKAI_API_KEY"),
        extra_env: None,
        detect: true,
    },
    // Batch 3: endpoint auth shape verified live (401 without key =
    // correct OpenAI-compatible path + Bearer scheme). Default models
    // are vendor flagships; confirm with a key via `providers test`.
    ProviderDef {
        kind: ProviderKind::Xai,
        id: "xai",
        aliases: &["grok", "x-ai"],
        default_url: "https://api.x.ai/v1",
        default_model: "grok-4",
        key_var: Some("XAI_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Nebius,
        id: "nebius",
        aliases: &["nebius-ai"],
        default_url: "https://api.studio.nebius.com/v1",
        default_model: "meta-llama/Llama-3.3-70B-Instruct",
        key_var: Some("NEBIUS_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Moonshot,
        id: "moonshot",
        aliases: &["kimi"],
        default_url: "https://api.moonshot.ai/v1",
        default_model: "kimi-k2-0711-preview",
        key_var: Some("MOONSHOT_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::MiniMax,
        id: "minimax",
        aliases: &[],
        default_url: "https://api.minimax.io/v1",
        default_model: "MiniMax-M2",
        key_var: Some("MINIMAX_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Upstage,
        id: "upstage",
        aliases: &["solar"],
        default_url: "https://api.upstage.ai/v1",
        default_model: "solar-pro",
        key_var: Some("UPSTAGE_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Hyperbolic,
        id: "hyperbolic",
        aliases: &[],
        default_url: "https://api.hyperbolic.xyz/v1",
        default_model: "meta-llama/Llama-3.3-70B-Instruct",
        key_var: Some("HYPERBOLIC_API_KEY"),
        extra_env: None,
        detect: true,
    },
    // Batch 5: endpoint auth shapes verified live (401 without key =
    // correct OpenAI-compatible path + Bearer scheme). Defaults are
    // vendor flagships; SambaNova's default came from its live catalog.
    // Ark (Volcengine) addresses models via endpoint IDs: override with
    // UTHARNESS_MODEL=<ep-...> when using a provisioned endpoint.
    ProviderDef {
        kind: ProviderKind::Zhipu,
        id: "zhipu",
        aliases: &["zai", "glm"],
        default_url: "https://api.z.ai/api/paas/v4",
        default_model: "glm-4.5",
        key_var: Some("ZHIPU_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::DashScope,
        id: "dashscope",
        aliases: &["qwen", "alibaba"],
        default_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        default_model: "qwen-max",
        key_var: Some("DASHSCOPE_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Volcengine,
        id: "volcengine",
        aliases: &["ark", "doubao"],
        default_url: "https://ark.cn-beijing.volces.com/api/v3",
        default_model: "doubao-seed-1-6",
        key_var: Some("ARK_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::StepFun,
        id: "stepfun",
        aliases: &["step"],
        default_url: "https://api.stepfun.com/v1",
        default_model: "step-2",
        key_var: Some("STEPFUN_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::SiliconFlow,
        id: "siliconflow",
        aliases: &["silicon"],
        default_url: "https://api.siliconflow.cn/v1",
        default_model: "deepseek-ai/DeepSeek-V3",
        key_var: Some("SILICONFLOW_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::SambaNova,
        id: "sambanova",
        aliases: &["samba"],
        default_url: "https://api.sambanova.ai/v1",
        default_model: "DeepSeek-V3.1",
        key_var: Some("SAMBANOVA_API_KEY"),
        extra_env: None,
        detect: true,
    },
    // Batch 6: endpoint shapes verified live. Defaults below come from
    // live catalog listings where available (Novita, Vercel, Chutes,
    // OVHcloud); the rest are vendor flagships to confirm with a key.
    ProviderDef {
        kind: ProviderKind::Novita,
        id: "novita",
        aliases: &[],
        default_url: "https://api.novita.ai/v3/openai",
        default_model: "zai-org/glm-5.3-flash",
        key_var: Some("NOVITA_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::HuggingFace,
        id: "huggingface",
        aliases: &["hf", "hugging-face"],
        default_url: "https://router.huggingface.co/v1",
        default_model: "Qwen/Qwen3-32B",
        key_var: Some("HF_TOKEN"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Vercel,
        id: "vercel",
        aliases: &["vercel-gateway", "ai-gateway"],
        default_url: "https://ai-gateway.vercel.sh/v1",
        default_model: "alibaba/qwen-3-14b",
        key_var: Some("VERCEL_AI_GATEWAY_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Chutes,
        id: "chutes",
        aliases: &["chute"],
        default_url: "https://llm.chutes.ai/v1",
        default_model: "Qwen/Qwen3.5-397B-A17B-TEE",
        key_var: Some("CHUTES_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::OvhCloud,
        id: "ovhcloud",
        aliases: &["ovh"],
        default_url: "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1",
        default_model: "Qwen3.5-397B-A17B",
        key_var: Some("OVH_AI_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Mimo,
        id: "mimo",
        aliases: &["xiaomi"],
        default_url: "https://api.xiaomimimo.com/v1",
        default_model: "mimo-v2.5",
        key_var: Some("MIMO_API_KEY"),
        extra_env: None,
        detect: true,
    },
    ProviderDef {
        kind: ProviderKind::Custom,
        id: "custom",
        aliases: &["openai-compatible"],
        default_url: "http://127.0.0.1:8000/v1",
        default_model: "default",
        key_var: Some("UTHARNESS_API_KEY"),
        extra_env: None,
        detect: false,
    },
];

impl ProviderKind {
    pub fn parse(value: &str) -> Result<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        PROVIDERS
            .iter()
            .find(|def| def.id == normalized || def.aliases.contains(&normalized.as_str()))
            .map(|def| def.kind.clone())
            .with_context(|| format!("unsupported provider '{value}'"))
    }
    pub fn id(&self) -> &'static str {
        self.def().id
    }
    fn def(&self) -> &'static ProviderDef {
        PROVIDERS
            .iter()
            .find(|def| def.kind == *self)
            .expect("provider table covers every ProviderKind")
    }
    fn defaults(&self) -> (&'static str, &'static str, Option<&'static str>) {
        let def = self.def();
        (def.default_url, def.default_model, def.key_var)
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
        } else {
            autodetect_provider()?.ok_or_else(|| {
                anyhow::anyhow!(
                    "no AI gateway is configured; set UTHARNESS_PROVIDER or a supported provider API key"
                )
            })?
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
    PROVIDERS
        .iter()
        .map(|def| Gateway::status_from_environment(def.kind.clone()))
        .collect()
}

/// Key variable for a provider id or alias (e.g. "nim" -> "NVIDIA_API_KEY").
/// Used by the CLI so only this table maps names to credential variables.
pub fn key_variable(name: &str) -> Option<&'static str> {
    ProviderKind::parse(name)
        .ok()
        .map(|kind| kind.def().key_var)?
}

/// Provider ids in display order, for `providers env` and docs.
pub fn provider_ids() -> Vec<&'static str> {
    PROVIDERS.iter().map(|def| def.id).collect()
}

/// Credential variables in display order, with `+EXTRA` hints appended
/// (e.g. `CLOUDFLARE_API_TOKEN+CLOUDFLARE_ACCOUNT_ID`).
pub fn key_variables() -> Vec<String> {
    let mut out = Vec::new();
    for def in PROVIDERS {
        if let Some(key) = def.key_var {
            let entry = match def.extra_env {
                Some(extra) => format!("{key}+{extra}"),
                None => key.to_string(),
            };
            if !out.contains(&entry) {
                out.push(entry);
            }
        }
    }
    out
}

fn autodetect_provider() -> Result<Option<ProviderKind>> {
    for def in PROVIDERS {
        if def.detect {
            if let Some(key) = def.key_var {
                if environment_has_value(key) {
                    return Ok(Some(def.kind.clone()));
                }
            }
        }
    }
    Ok(None)
}

pub fn has_provider_configuration() -> bool {
    std::env::var("UTHARNESS_PROVIDER").is_ok_and(|value| !value.trim().is_empty())
        || PROVIDERS.iter().any(|def| {
            def.key_var
                .is_some_and(|name| name != "UTHARNESS_API_KEY" && environment_has_value(name))
        })
        || environment_has_value("UTHARNESS_API_KEY")
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
    fn provider_table_is_consistent() {
        use std::collections::HashSet;
        // Every id and alias resolves to its row's kind; new rows are
        // covered automatically, so this test never needs new asserts.
        let mut ids = HashSet::new();
        for def in PROVIDERS {
            assert!(ids.insert(def.id), "duplicate provider id");
            assert_eq!(ProviderKind::parse(def.id).unwrap(), def.kind);
            for alias in def.aliases {
                assert_eq!(ProviderKind::parse(alias).unwrap(), def.kind);
            }
            if def.kind == ProviderKind::Ollama {
                assert!(def.key_var.is_none());
            } else {
                assert!(def.key_var.is_some(), "{}", def.id);
            }
            assert!(validate_base_url(def.default_url).is_ok(), "{}", def.id);
        }
        assert!(ProviderKind::parse("unknown").is_err());
        assert_eq!(
            provider_ids(),
            PROVIDERS.iter().map(|def| def.id).collect::<Vec<_>>()
        );
        assert!(key_variables().iter().any(|v| v == "GROQ_API_KEY"));
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
