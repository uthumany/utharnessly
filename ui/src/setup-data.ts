export type SetupMode = 'quick' | 'full' | 'blank';

export type ProviderOption = {
  id: string;
  label: string;
  description: string;
  model: string;
  key?: string;
};

export type ToolOption = {
  id: string;
  label: string;
  description: string;
  risk: 'safe' | 'ask';
};

export const modes: Array<{ id: SetupMode; label: string; description: string }> = [
  { id: 'quick', label: 'Quick Setup', description: 'provider, model, and recommended safe capabilities' },
  { id: 'full', label: 'Full Setup', description: 'choose every available capability yourself' },
  { id: 'blank', label: 'Blank Slate', description: 'offline planner with workspace reading only' }
];

export const providers: ProviderOption[] = [
  { id: 'openrouter', label: 'OpenRouter', description: 'pay-per-use OpenAI-compatible aggregator', model: 'openrouter/free', key: 'OPENROUTER_API_KEY' },
  { id: 'openai', label: 'OpenAI', description: 'direct OpenAI API', model: 'gpt-4o-mini', key: 'OPENAI_API_KEY' },
  { id: 'groq', label: 'Groq', description: 'low-latency OpenAI-compatible API', model: 'llama-3.3-70b-versatile', key: 'GROQ_API_KEY' },
  { id: 'together', label: 'Together AI', description: 'open-model inference API', model: 'meta-llama/Llama-3.3-70B-Instruct-Turbo', key: 'TOGETHER_API_KEY' },
  { id: 'deepseek', label: 'DeepSeek', description: 'direct chat and coding models', model: 'deepseek-chat', key: 'DEEPSEEK_API_KEY' },
  { id: 'fireworks', label: 'Fireworks AI', description: 'OpenAI-compatible model API', model: 'accounts/fireworks/models/llama-v3p3-70b-instruct', key: 'FIREWORKS_API_KEY' },
  { id: 'nvidia', label: 'NVIDIA NIM', description: 'hosted Nemotron models on build.nvidia.com', model: 'nvidia/nemotron-3-super-120b-a12b', key: 'NVIDIA_API_KEY' },
  { id: 'ollama', label: 'Ollama', description: 'local model server; no API key', model: 'qwen2.5-coder:7b' },
  { id: 'custom', label: 'Custom endpoint', description: 'OpenAI-compatible endpoint via UTHARNESS_PROVIDER_URL', model: 'default', key: 'UTHARNESS_API_KEY' }
];

export const tools: ToolOption[] = [
  { id: 'workspace_read', label: 'Workspace inspection', description: 'list and read files', risk: 'safe' },
  { id: 'git_inspection', label: 'Git inspection', description: 'status and diff', risk: 'safe' },
  { id: 'terminal', label: 'Terminal & processes', description: 'bounded shell commands; approval required', risk: 'ask' },
  { id: 'file_write', label: 'File changes', description: 'write and patch files; approval required', risk: 'ask' },
  { id: 'skills', label: 'Skills', description: 'list, inspect, install, and run skills', risk: 'safe' },
  { id: 'memory', label: 'Persistent memory', description: 'project notes across sessions', risk: 'safe' },
  { id: 'session_search', label: 'Session search', description: 'search saved local conversations', risk: 'safe' },
  { id: 'task_planning', label: 'Task planning', description: 'checkpoints and structured agent plans', risk: 'safe' }
];

export const recommendedTools = ['workspace_read', 'git_inspection', 'skills', 'memory'];
