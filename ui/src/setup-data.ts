export type SetupMode = 'quick' | 'full' | 'developer' | 'local_ai' | 'custom' | 'blank' | 'import' | 'exit';
export type AuthMethod = 'api_key' | 'oauth' | 'environment' | 'skip';
export type ProviderOption = { id: string; label: string; description: string; model: string; key?: string };
export type ToolOption = { id: string; label: string; description: string; risk: 'safe' | 'ask' };
export type ScanComponent = { id: string; label: string; state: 'AVAILABLE' | 'MISSING' | 'BROKEN' | 'OPTIONAL'; required: boolean; version?: string; installHint?: string };
export type EnvironmentReport = { os: string; architecture: string; shell: string; terminal: string; packageManager?: string; components: ScanComponent[] };

export const modes: Array<{ id: SetupMode; label: string; description: string }> = [
  { id: 'quick', label: 'Quick Start', description: 'provider, model, and safe defaults' },
  { id: 'full', label: 'Full Setup', description: 'choose every runtime capability' },
  { id: 'developer', label: 'Developer Setup', description: 'coding, Git, terminal, skills, memory, and agents' },
  { id: 'local_ai', label: 'Local AI Setup', description: 'Ollama with no hosted API key' },
  { id: 'custom', label: 'Custom Provider', description: 'OpenAI-compatible URL, key, and model' },
  { id: 'blank', label: 'Blank Slate', description: 'offline planner and workspace reading only' },
  { id: 'import', label: 'Import Configuration', description: 'validate an existing utharness.json' },
  { id: 'exit', label: 'Exit', description: 'leave without changing configuration' }
];

export const providers: ProviderOption[] = [
  { id: 'openrouter', label: 'OpenRouter', description: 'OpenAI-compatible model aggregator', model: 'openrouter/free', key: 'OPENROUTER_API_KEY' },
  { id: 'openai', label: 'OpenAI', description: 'direct OpenAI API', model: 'gpt-4o-mini', key: 'OPENAI_API_KEY' },
  { id: 'groq', label: 'Groq', description: 'low-latency hosted inference', model: 'groq/compound-mini', key: 'GROQ_API_KEY' },
  { id: 'together', label: 'Together AI', description: 'open-model inference', model: 'meta-llama/Llama-3.3-70B-Instruct-Turbo', key: 'TOGETHER_API_KEY' },
  { id: 'deepseek', label: 'DeepSeek', description: 'chat and coding models', model: 'deepseek-chat', key: 'DEEPSEEK_API_KEY' },
  { id: 'fireworks', label: 'Fireworks AI', description: 'OpenAI-compatible model API', model: 'accounts/fireworks/models/llama-v3p3-70b-instruct', key: 'FIREWORKS_API_KEY' },
  { id: 'nvidia', label: 'NVIDIA NIM', description: 'hosted Nemotron models', model: 'nvidia/nemotron-3-super-120b-a12b', key: 'NVIDIA_API_KEY' },
  { id: 'mistral', label: 'Mistral AI', description: 'Mistral models', model: 'mistral-small-latest', key: 'MISTRAL_API_KEY' },
  { id: 'cerebras', label: 'Cerebras', description: 'ultra-fast hosted inference', model: 'qwen-3.8-27b', key: 'CEREBRAS_API_KEY' },
  { id: 'cohere', label: 'Cohere', description: 'Command models via compat API', model: 'command-a-03-2025', key: 'COHERE_API_KEY' },
  { id: 'cometapi', label: 'CometAPI', description: 'multi-model gateway', model: 'gpt-4o-mini', key: 'COMETAPI_API_KEY' },
  { id: 'cloudflare', label: 'Cloudflare', description: 'Workers AI · set CLOUDFLARE_ACCOUNT_ID too', model: '@cf/qwen/qwen3.8-27b', key: 'CLOUDFLARE_API_TOKEN' },
  { id: 'ollama', label: 'Ollama', description: 'local model server; no API key', model: 'qwen2.5-coder:7b' },
  { id: 'ollama-cloud', label: 'Ollama Cloud', description: 'hosted Ollama models', model: 'nemotron-3-ultra', key: 'OLLAMA_API_KEY' },
  { id: 'seekai', label: 'SeekAI', description: 'aggregator · MiMo models', model: 'mimo-v2.5', key: 'SEEKAI_API_KEY' },
  { id: 'xai', label: 'xAI', description: 'Grok models', model: 'grok-4', key: 'XAI_API_KEY' },
  { id: 'nebius', label: 'Nebius', description: 'GPU cloud inference', model: 'meta-llama/Llama-3.3-70B-Instruct', key: 'NEBIUS_API_KEY' },
  { id: 'moonshot', label: 'Moonshot', description: 'Kimi models', model: 'kimi-k2-0711-preview', key: 'MOONSHOT_API_KEY' },
  { id: 'minimax', label: 'MiniMax', description: 'MiniMax models', model: 'MiniMax-M2', key: 'MINIMAX_API_KEY' },
  { id: 'upstage', label: 'Upstage', description: 'Solar models', model: 'solar-pro', key: 'UPSTAGE_API_KEY' },
  { id: 'hyperbolic', label: 'Hyperbolic', description: 'GPU marketplace inference', model: 'meta-llama/Llama-3.3-70B-Instruct', key: 'HYPERBOLIC_API_KEY' },
  { id: 'zhipu', label: 'Zhipu', description: 'GLM models', model: 'glm-4.5', key: 'ZHIPU_API_KEY' },
  { id: 'dashscope', label: 'DashScope', description: 'Qwen models · Alibaba', model: 'qwen-max', key: 'DASHSCOPE_API_KEY' },
  { id: 'volcengine', label: 'Volcengine', description: 'Doubao models via Ark', model: 'doubao-seed-1-6', key: 'ARK_API_KEY' },
  { id: 'stepfun', label: 'StepFun', description: 'Step models', model: 'step-2', key: 'STEPFUN_API_KEY' },
  { id: 'siliconflow', label: 'SiliconFlow', description: 'SiliconFlow inference', model: 'deepseek-ai/DeepSeek-V3', key: 'SILICONFLOW_API_KEY' },
  { id: 'sambanova', label: 'SambaNova', description: 'SambaNova inference', model: 'DeepSeek-V3.1', key: 'SAMBANOVA_API_KEY' },
  { id: 'novita', label: 'Novita', description: 'Novita AI inference', model: 'zai-org/glm-5.3-flash', key: 'NOVITA_API_KEY' },
  { id: 'huggingface', label: 'HuggingFace', description: 'HF Inference Router', model: 'Qwen/Qwen3-32B', key: 'HF_TOKEN' },
  { id: 'vercel', label: 'Vercel', description: 'Vercel AI Gateway', model: 'alibaba/qwen-3-14b', key: 'VERCEL_AI_GATEWAY_API_KEY' },
  { id: 'chutes', label: 'Chutes', description: 'Chutes inference', model: 'Qwen/Qwen3.5-397B-A17B-TEE', key: 'CHUTES_API_KEY' },
  { id: 'ovhcloud', label: 'OVHcloud', description: 'OVHcloud AI Endpoints', model: 'Qwen3.5-397B-A17B', key: 'OVH_AI_API_KEY' },
  { id: 'mimo', label: 'MiMo', description: 'Xiaomi MiMo models', model: 'mimo-v2.5', key: 'MIMO_API_KEY' },
  { id: 'teamorouter', label: 'TeamoRouter', description: 'multi-model router', model: 'gpt-5.5', key: 'TEAMOROUTER_API_KEY' },
  { id: 'perplexity', label: 'Perplexity', description: 'answer engine models', model: 'sonar', key: 'PERPLEXITY_API_KEY' },
  { id: 'gemini', label: 'Gemini', description: 'Google Gemini API', model: 'gemini-2.5-flash', key: 'GEMINI_API_KEY' },
  { id: 'custom', label: 'Custom endpoint', description: 'OpenAI-compatible endpoint', model: 'default', key: 'UTHARNESS_API_KEY' }
];

export const authMethods: Array<{ id: AuthMethod; label: string; description: string }> = [
  { id: 'api_key', label: 'API Key', description: 'masked input; private secrets.env' },
  { id: 'oauth', label: 'OAuth', description: 'when supported by the provider adapter' },
  { id: 'environment', label: 'Environment Variable', description: 'reuse an existing provider variable' },
  { id: 'skip', label: 'Skip', description: 'save incomplete setup and validate later' }
];

export const tools: ToolOption[] = [
  { id: 'workspace_read', label: 'Files & repository search', description: 'read, list, and search files', risk: 'safe' },
  { id: 'git_inspection', label: 'Git inspection', description: 'status, diff, and history', risk: 'safe' },
  { id: 'terminal', label: 'Terminal & processes', description: 'bounded commands with approval', risk: 'ask' },
  { id: 'file_write', label: 'File editing', description: 'write and patch with approval', risk: 'ask' },
  { id: 'skills', label: 'Skills & MCP', description: 'validated extension registries', risk: 'safe' },
  { id: 'memory', label: 'Persistent memory', description: 'project notes across sessions', risk: 'safe' },
  { id: 'session_search', label: 'Sessions', description: 'search local conversation history', risk: 'safe' },
  { id: 'task_planning', label: 'Agents & planning', description: 'checkpoints and bounded plans', risk: 'safe' },
  { id: 'desktop', label: 'Desktop control', description: 'screenshot, click, and type with approval', risk: 'ask' }
];

export const recommendedTools = ['workspace_read', 'git_inspection', 'skills', 'memory'];
export const developerTools = tools.map(tool => tool.id);
export function progress(completed: number, total: number) { return total <= 0 ? 100 : Math.max(0, Math.min(100, Math.round(completed / total * 100))); }
