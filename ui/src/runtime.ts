import chokidar, { type FSWatcher } from 'chokidar';
import { execa } from 'execa';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { z } from 'zod';
import { runtimeBinary } from './runtime-binary.js';
import { parseModelCatalog } from './setup.js';
import type { Message, RuntimeSnapshot, ToolCard } from './types.js';

const runtimeSchema = z.object({
  workspace: z.string(),
  permission: z.string(),
  provider: z.string(),
  model: z.string(),
  context: z.string(),
  network: z.string(),
  projectSpecific: z.boolean(),
  platform: z.string(),
  androidVersion: z.string(),
  prefix: z.string(),
  termuxApi: z.string(),
  storage: z.string(),
  messages: z.array(z.object({
    id: z.string(),
    role: z.enum(['utharness', 'you', 'system', 'agent', 'tool', 'memory', 'error']),
    text: z.string(),
    time: z.string(),
    tool: z.object({
      id: z.string(),
      name: z.string(),
      icon: z.string(),
      state: z.enum(['waiting', 'running', 'completed', 'error', 'approval']),
      result: z.string(),
      metric: z.string(),
      elapsed: z.string()
    }).optional()
  })),
  git: z.object({ branch: z.string(), modified: z.number(), untracked: z.number(), additions: z.number(), deletions: z.number() }),
  activeAgents: z.number()
});

const now = () => new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
const id = () => `${Date.now()}-${Math.random().toString(16).slice(2)}`;

// Saved provider/model resolution for the snapshot header. Mirrors the CLI
// precedence (explicit env > workspace utharness.json > global config.yaml):
// without this, opening the TUI anywhere dropped the setup choice.
export async function resolveSavedSelection(cwd = process.cwd(), utharnessHome = process.env.UTHARNESS_HOME ?? path.join(os.homedir(), '.utharness')): Promise<{ provider?: string; model?: string }> {
  try {
    const raw = await fs.readFile(path.join(cwd, 'utharness.json'), 'utf8');
    const config = JSON.parse(raw) as { schemaVersion?: number; provider?: string; model?: string };
    if (config.schemaVersion === 1 && config.provider && config.provider !== 'offline' && config.model) {
      return { provider: config.provider, model: config.model };
    }
  } catch {
    // No workspace config; fall through to the global file.
  }
  try {
    const raw = await fs.readFile(path.join(utharnessHome, 'config.yaml'), 'utf8');
    let provider: string | undefined; let model: string | undefined;
    for (const line of raw.split('\n')) {
      const separator = line.indexOf(':');
      if (separator < 0) continue;
      const key = line.slice(0, separator).trim();
      if (key !== 'provider' && key !== 'model') continue;
      try {
        const value = JSON.parse(line.slice(separator + 1).trim()) as unknown;
        if (typeof value === 'string' && value) {
          if (key === 'provider') provider = value; else model = value;
        }
      } catch {
        // Skip malformed lines; a partial file still yields nothing.
      }
    }
    if (provider && provider !== 'offline' && model) return { provider, model };
  } catch {
    // No global config either; caller falls back to env/autodetect.
  }
  return {};
}

const initialMessages = (): Message[] => [
  { id: id(), role: 'utharness', text: 'Ready. Ask a question, reference @files, or run a slash command.', time: now() }
];

async function commandOutput(command: string, args: string[], cwd: string): Promise<string> {
  const result = await execa(command, args, { cwd, reject: false, timeout: 2_500 });
  return result.stdout.trim();
}

export async function loadSnapshot(cwd = process.cwd()): Promise<RuntimeSnapshot> {
  const home = os.homedir();
  const [branch, gitRoot, porcelain, diffStats, androidVersion, apiCommand] = await Promise.all([
    commandOutput('git', ['branch', '--show-current'], cwd),
    commandOutput('git', ['rev-parse', '--show-toplevel'], cwd),
    commandOutput('git', ['status', '--porcelain'], cwd),
    commandOutput('git', ['diff', '--numstat'], cwd),
    commandOutput('getprop', ['ro.build.version.release'], cwd),
    commandOutput('sh', ['-lc', 'command -v termux-battery-status'], cwd)
  ]);
  const isTermux = Boolean(process.env.TERMUX_VERSION || process.env.PREFIX?.includes('com.termux'));
  const workspacePath = gitRoot || cwd;
  const workspace = workspacePath.startsWith(home) ? `~${workspacePath.slice(home.length)}` : workspacePath;
  const saved = await resolveSavedSelection(cwd);
  const provider = process.env.UTHARNESS_PROVIDER ?? saved.provider ?? (process.env.OPENROUTER_API_KEY ? 'openrouter' : 'offline');
  const storagePath = path.join(home, 'storage');
  let storage = 'sandbox';
  try {
    await fs.access(storagePath);
    storage = 'shared storage linked';
  } catch {
    // Shared storage is optional on Termux.
  }
  const snapshot = {
    workspace,
    permission: process.env.UTHARNESS_PERMISSION ?? 'offline (default deny)',
    provider,
    model: process.env.UTHARNESS_MODEL ?? (process.env.UTHARNESS_PROVIDER ? undefined : saved.model) ?? 'gpt-4o-mini',
    context: process.env.UTHARNESS_CONTEXT ?? '128K context left',
    network: process.env.OPENROUTER_API_KEY ? 'connected' : 'offline',
    projectSpecific: Boolean(gitRoot),
    platform: isTermux ? 'termux' : process.platform,
    androidVersion: androidVersion || 'n/a',
    prefix: process.env.PREFIX ?? '',
    termuxApi: apiCommand ? 'available' : 'optional/missing',
    storage,
    git: parseGitSnapshot(branch, porcelain, diffStats),
    activeAgents: Number.parseInt(process.env.UTHARNESS_ACTIVE_AGENTS ?? '0', 10) || 0,
    messages: initialMessages()
  } satisfies RuntimeSnapshot;
  return runtimeSchema.parse(snapshot);
}

export function parseGitSnapshot(branch: string, porcelain: string, diffStats: string) {
  const statusLines = porcelain.split(/\r?\n/).filter(Boolean);
  const untracked = statusLines.filter(line => line.startsWith('??')).length;
  const modified = statusLines.length - untracked;
  let additions = 0;
  let deletions = 0;
  for (const line of diffStats.split(/\r?\n/).filter(Boolean)) {
    const [added, removed] = line.split(/\s+/);
    if (added !== '-') additions += Number.parseInt(added ?? '0', 10) || 0;
    if (removed !== '-') deletions += Number.parseInt(removed ?? '0', 10) || 0;
  }
  return { branch: branch || 'no-branch', modified, untracked, additions, deletions };
}

export async function runSkillCommand(args: string[], cwd = process.cwd()): Promise<string> {
  const binary = runtimeBinary();
  try {
    await fs.access(binary);
    const result = await execa(binary, ['skills', ...args], { cwd, reject: false, timeout: 8_000 });
    return (result.stdout || result.stderr).trim() || 'Skill Engine returned no output.';
  } catch (error) {
    return `Skill Engine unavailable: ${error instanceof Error ? error.message : String(error)}`;
  }
}

export type AgentOptions = { provider?: string; model?: string; signal?: AbortSignal };

export async function runRuntimeCommand(args: string[], cwd = process.cwd(), provider?: string): Promise<string> {
  const result = await execa(runtimeBinary(cwd), args, { cwd, reject: false, timeout: 30_000, env: provider ? { UTHARNESS_PROVIDER: provider } : {} });
  if (result.exitCode !== 0) throw new Error(result.stderr.trim() || `Runtime command failed (${result.exitCode}).`);
  return result.stdout.trim();
}

export async function loadModelCatalog(cwd = process.cwd(), provider?: string) {
  return parseModelCatalog(await runRuntimeCommand(['models', 'list', '--json'], cwd, provider));
}

export async function submitAgent(prompt: string, cwd = process.cwd(), options: AgentOptions = {}): Promise<{ text: string; tool?: ToolCard }> {
  const binary = runtimeBinary(cwd);
  try {
    await fs.access(binary);
  } catch {
    throw new Error('Agent runtime unavailable. Reinstall UTHARNESS or set UTHARNESS_RUNTIME_BIN.');
  }
  const result = await execa(binary, ['agents', 'run', prompt, '--workspace', cwd], {
    cwd, reject: false, timeout: 600_000, cancelSignal: options.signal, forceKillAfterDelay: 1000,
    env: { ...(options.provider ? { UTHARNESS_PROVIDER: options.provider } : {}), ...(options.model ? { UTHARNESS_MODEL: options.model } : {}) },
  });
  if (options.signal?.aborted) throw new Error('Agent operation cancelled.');
  if (result.exitCode !== 0) {
    throw new Error(result.stderr.trim() || `Agent runtime exited with status ${result.exitCode}.`);
  }
  const text = result.stdout.trim();
  if (!text) throw new Error('Agent runtime completed without output.');
  return { text };
}

/** Backwards-compatible alias: plain submission used to mean the agent. */
export const submitPrompt = submitAgent;

/** Conversational turn: plain composer text chats with the model instead
 *  of running a workspace inspection. Strips the `Uthy · provider/model`
 *  header line the CLI prints above the streamed reply. */
export async function submitChat(prompt: string, cwd = process.cwd(), options: AgentOptions = {}): Promise<{ text: string }> {
  const binary = runtimeBinary(cwd);
  try {
    await fs.access(binary);
  } catch {
    throw new Error('Chat runtime unavailable. Reinstall UTHARNESS or set UTHARNESS_RUNTIME_BIN.');
  }
  const result = await execa(binary, ['chat', prompt], {
    cwd, reject: false, timeout: 120_000, cancelSignal: options.signal, forceKillAfterDelay: 1000,
    env: { ...(options.provider ? { UTHARNESS_PROVIDER: options.provider } : {}), ...(options.model ? { UTHARNESS_MODEL: options.model } : {}) },
  });
  if (options.signal?.aborted) throw new Error('Chat operation cancelled.');
  if (result.exitCode !== 0) {
    throw new Error(result.stderr.trim() || `Chat runtime exited with status ${result.exitCode}.`);
  }
  const lines = result.stdout.trim().split('\n');
  if (lines.length && lines[0]?.includes('Uthy ·')) lines.shift();
  const text = lines.join('\n').trim();
  if (!text) throw new Error('Chat runtime completed without output.');
  return { text };
}

export function watchRuntime(cwd: string, onChange: () => void): FSWatcher {
  const watcher = chokidar.watch([path.join(cwd, 'UTHARNESS.md'), path.join(cwd, '.git', 'HEAD')], {
    ignoreInitial: true,
    persistent: false
  });
  watcher.on('all', (_event, changedPath) => {
    if (changedPath.endsWith('UTHARNESS.md') || changedPath.endsWith(path.join('.git', 'HEAD'))) onChange();
  });
  return watcher;
}
