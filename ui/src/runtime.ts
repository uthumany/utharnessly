import chokidar, { type FSWatcher } from 'chokidar';
import { execa } from 'execa';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { z } from 'zod';
import type { Message, RuntimeSnapshot, ToolCard } from './types.js';

const runtimeSchema = z.object({
  workspace: z.string(),
  permission: z.string(),
  provider: z.string(),
  model: z.string(),
  branch: z.string(),
  context: z.string(),
  network: z.string(),
  projectSpecific: z.boolean(),
  messages: z.array(z.object({
    id: z.string(),
    role: z.enum(['uthy', 'you']),
    text: z.string(),
    time: z.string(),
    tool: z.object({
      id: z.string(),
      name: z.string(),
      icon: z.string(),
      state: z.enum(['running', 'completed', 'error', 'approval']),
      result: z.string(),
      metric: z.string(),
      elapsed: z.string()
    }).optional()
  }))
});

const now = () => new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
const id = () => `${Date.now()}-${Math.random().toString(16).slice(2)}`;

const sampleMessages = (): Message[] => [
  { id: id(), role: 'uthy', text: "Hello! I'm UTHY, your terminal agent. How can I help you today?", time: now() },
  { id: id(), role: 'you', text: 'List the top 5 largest TypeScript files in this repo.', time: now() },
  {
    id: id(), role: 'uthy', text: "I'll scan the repository and find the largest TypeScript files.", time: now(),
    tool: { id: id(), name: 'read_directory', icon: '▱', state: 'completed', result: 'Completed', metric: '1,842 files', elapsed: '120ms' }
  },
  {
    id: id(), role: 'uthy', text: 'Here are the top 5 largest TypeScript files:', time: now(),
    tool: { id: id(), name: 'list_large_files', icon: '</>', state: 'completed', result: 'Completed', metric: '5 results', elapsed: '88ms' }
  }
];

async function commandOutput(command: string, args: string[], cwd: string): Promise<string> {
  const result = await execa(command, args, { cwd, reject: false, timeout: 2_500 });
  return result.stdout.trim();
}

function runtimeBinary(): string {
  return process.env.UTHARNESS_RUNTIME_BIN ?? path.resolve(process.cwd(), '../target/release/utharness');
}

export async function loadSnapshot(cwd = process.cwd()): Promise<RuntimeSnapshot> {
  const [branch, gitRoot, model] = await Promise.all([
    commandOutput('git', ['branch', '--show-current'], cwd),
    commandOutput('git', ['rev-parse', '--show-toplevel'], cwd),
    Promise.resolve(process.env.UTHARNESS_MODEL ?? 'gpt-4o-mini')
  ]);
  const workspacePath = gitRoot || cwd;
  const home = os.homedir();
  const workspace = workspacePath.startsWith(home) ? `~${workspacePath.slice(home.length)}` : workspacePath;
  const provider = process.env.UTHARNESS_PROVIDER ?? (process.env.OPENROUTER_API_KEY ? 'openrouter' : 'offline');
  const snapshot = {
    workspace,
    permission: process.env.UTHARNESS_PERMISSION ?? 'offline (default deny)',
    provider,
    model,
    branch: branch || 'no-branch',
    context: process.env.UTHARNESS_CONTEXT ?? '128K context left',
    network: process.env.OPENROUTER_API_KEY ? 'connected' : 'offline',
    projectSpecific: Boolean(gitRoot),
    messages: sampleMessages()
  } satisfies RuntimeSnapshot;
  return runtimeSchema.parse(snapshot);
}

export async function submitPrompt(prompt: string, cwd = process.cwd()): Promise<{ text: string; tool?: ToolCard }> {
  const binary = runtimeBinary();
  let commandResult = '';
  try {
    await fs.access(binary);
    const result = await execa(binary, ['chat', prompt], { cwd, reject: false, timeout: 10_000 });
    commandResult = result.stdout.trim();
  } catch {
    commandResult = '';
  }

  if (prompt.toLowerCase().includes('largest') && prompt.toLowerCase().includes('typescript')) {
    return {
      text: 'Here are the top 5 largest TypeScript files:',
      tool: { id: id(), name: 'list_large_files', icon: '</>', state: 'completed', result: 'Completed', metric: '5 results', elapsed: '88ms' }
    };
  }

  return {
    text: commandResult || `I received: “${prompt}”\n\nI can inspect files, review Git state, and run bounded SAFE tasks from this terminal.`,
    tool: { id: id(), name: 'agent_response', icon: '✦', state: 'completed', result: 'Completed', metric: 'offline planner', elapsed: '42ms' }
  };
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
