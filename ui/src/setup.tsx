import React, { useMemo, useState } from 'react';
import { Box, Text, useApp, useInput, useStdout } from 'ink';
import { execa } from 'execa';
import path from 'node:path';
import { modes, providers, recommendedTools, tools, type SetupMode } from './setup-data.js';

type Stage = 'welcome' | 'mode' | 'provider' | 'tools' | 'review' | 'saving' | 'done' | 'error';
const cyan = '#25d9cb';
const purple = '#db79ff';
const muted = '#8b93a7';

function runtimeBinary() {
  return process.env.UTHARNESS_RUNTIME_BIN ?? path.resolve(process.cwd(), '../target/release/utharness');
}

function List({ items, selected, marked }: { items: Array<{ id: string; label: string; description: string }>; selected: number; marked?: Set<string> }) {
  return <Box flexDirection="column" marginTop={1}>
    {items.map((item, index) => <Text key={item.id} color={index === selected ? cyan : undefined}>
      {index === selected ? ' ➤ ' : '   '}{marked ? `[${marked.has(item.id) ? '●' : ' '}]` : `(${index === selected ? '●' : ' '})`} <Text bold={index === selected}>{item.label}</Text> <Text color={muted}>— {item.description}</Text>
    </Text>)}
  </Box>;
}

export function SetupApp() {
  const { exit } = useApp();
  const { stdout } = useStdout();
  const [stage, setStage] = useState<Stage>('welcome');
  const [selected, setSelected] = useState(0);
  const [mode, setMode] = useState<SetupMode>('quick');
  const [provider, setProvider] = useState('openrouter');
  const [enabled, setEnabled] = useState(new Set(recommendedTools));
  const [error, setError] = useState('');
  const columns = Math.max(40, stdout.columns ?? 80);
  const compact = columns < 70;
  const providerInfo = providers.find(item => item.id === provider) ?? providers[0]!;
  const selectedTools = useMemo(() => tools.filter(tool => enabled.has(tool.id)), [enabled]);
  const list = stage === 'mode' ? modes : stage === 'provider' ? providers : stage === 'tools' ? tools : [];

  const save = async () => {
    setStage('saving');
    const chosenTools = mode === 'blank' ? ['workspace_read'] : [...enabled];
    const args = ['setup', '--non-interactive', '--mode', mode, '--provider', mode === 'blank' ? 'offline' : provider, '--model', mode === 'blank' ? 'deterministic-planner' : providerInfo.model, '--tools', chosenTools.join(',') || 'none'];
    const result = await execa(runtimeBinary(), args, { cwd: process.cwd(), reject: false });
    if (result.exitCode === 0) setStage('done');
    else { setError(result.stderr || result.stdout || 'Setup failed'); setStage('error'); }
  };

  useInput((input, key) => {
    if (key.ctrl && input === 'c') { exit(); return; }
    if (stage === 'welcome' && key.return) { setStage('mode'); setSelected(0); return; }
    if (stage === 'done' && key.return) { exit(); return; }
    if (stage === 'error' && (key.return || key.escape)) { setStage('review'); return; }
    if (!list.length && stage !== 'review') return;
    if (key.upArrow) setSelected(value => Math.max(0, value - 1));
    if (key.downArrow) setSelected(value => Math.min(list.length - 1, value + 1));
    if (stage === 'tools' && input === ' ') {
      const id = tools[selected]?.id;
      if (id) setEnabled(current => { const next = new Set(current); next.has(id) ? next.delete(id) : next.add(id); return next; });
      return;
    }
    if (key.escape || key.leftArrow) {
      if (stage === 'mode') setStage('welcome');
      else if (stage === 'provider') { setStage('mode'); setSelected(modes.findIndex(item => item.id === mode)); }
      else if (stage === 'tools') { setStage(mode === 'blank' ? 'mode' : 'provider'); setSelected(0); }
      else if (stage === 'review') { setStage(mode === 'full' ? 'tools' : mode === 'blank' ? 'mode' : 'provider'); setSelected(0); }
      return;
    }
    if (stage === 'review' && key.return) { void save(); return; }
    if (!key.return && input !== ' ') return;
    if (stage === 'mode') {
      const next = modes[selected]?.id ?? 'quick'; setMode(next);
      if (next === 'blank') { setEnabled(new Set(['workspace_read'])); setStage('review'); }
      else { setStage('provider'); setSelected(0); }
    } else if (stage === 'provider') {
      setProvider(providers[selected]?.id ?? 'openrouter');
      if (mode === 'full') { setStage('tools'); setSelected(0); } else setStage('review');
    } else if (stage === 'tools' && key.return) setStage('review');
  });

  return <Box flexDirection="column" paddingX={compact ? 1 : 3} paddingY={1} width={columns}>
    <Text bold color={purple}>UTHARNESS · AGENT SETUP</Text>
    <Text color={muted}>{'─'.repeat(Math.max(20, Math.min(columns - (compact ? 2 : 6), 76)))}</Text>
    {stage === 'welcome' ? <Box flexDirection="column" marginTop={1}>
      <Text bold>Welcome to Utharness Agent Setup</Text>
      <Text>This wizard configures the real local runtime, model route, and permission-gated capabilities.</Text>
      <Text color={muted}>Secrets stay in environment variables and are never written to configuration.</Text>
      <Text color={cyan}>Press Enter to continue…</Text>
    </Box> : null}
    {stage === 'mode' ? <><Text bold>How would you like to set up Utharness?</Text><List items={modes} selected={selected} /></> : null}
    {stage === 'provider' ? <><Text bold>Select an available provider</Text><Text color={muted}>Configured keys are detected when the agent starts.</Text><List items={providers} selected={selected} /></> : null}
    {stage === 'tools' ? <><Text bold>Tools for the CLI agent</Text><List items={tools} selected={selected} marked={enabled} /></> : null}
    {stage === 'review' ? <Box flexDirection="column" marginTop={1}>
      <Text bold>Review configuration</Text>
      <Text>Mode       <Text color={cyan}>{mode}</Text></Text>
      <Text>Provider   <Text color={cyan}>{mode === 'blank' ? 'offline' : providerInfo.label}</Text></Text>
      <Text>Model      <Text color={cyan}>{mode === 'blank' ? 'deterministic-planner' : providerInfo.model}</Text></Text>
      <Text>Credential <Text color={providerInfo.key && !process.env[providerInfo.key] ? '#ffbe55' : '#69d26f'}>{mode === 'blank' ? 'not required' : providerInfo.key ? (process.env[providerInfo.key] ? `${providerInfo.key} detected` : `${providerInfo.key} required at runtime`) : 'not required'}</Text></Text>
      <Text>Tools      <Text color={cyan}>{(mode === 'blank' ? ['workspace_read'] : selectedTools.map(item => item.id)).join(', ') || 'none'}</Text></Text>
      <Text color={muted}>Writes and terminal commands remain approval-gated.</Text>
      <Text color={cyan}>Press Enter to save utharness.json.</Text>
    </Box> : null}
    {stage === 'saving' ? <Text color={cyan}>Saving validated runtime configuration…</Text> : null}
    {stage === 'done' ? <Box flexDirection="column"><Text color="#69d26f" bold>✓ Setup complete</Text><Text>utharness.json now controls the selected provider, model, tools, and permission mode.</Text><Text color={cyan}>Press Enter to exit.</Text></Box> : null}
    {stage === 'error' ? <Box flexDirection="column"><Text color="#ff6b6b" bold>Setup failed</Text><Text>{error}</Text><Text color={muted}>Press Enter to return to review.</Text></Box> : null}
    {['mode', 'provider', 'tools'].includes(stage) ? <Text color={muted}>↑↓ navigate · {stage === 'tools' ? 'Space toggle · ' : ''}Enter select · Esc previous · Ctrl+C exit</Text> : null}
  </Box>;
}
