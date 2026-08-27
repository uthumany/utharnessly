import React, { useEffect, useMemo, useState } from 'react';
import { Box, Text, useApp, useInput, useStdout } from 'ink';
import { TextInput } from '@inkjs/ui';
import type { FSWatcher } from 'chokidar';
import { loadSnapshot, runSkillCommand, submitPrompt, watchRuntime } from './runtime.js';
import { Banner, getBreakpoint, getColorMode, getTermuxBreakpoint, MessageRow, palette, PersistentHeader, PromptFrame, SkillManager, StartupTips, StatusBar, WorkspaceWarning } from './components.js';
import type { CommandItem, Message, RuntimeSnapshot } from './types.js';

const commands: CommandItem[] = [
  { command: '/model', description: 'choose the active model' },
  { command: '/provider', description: 'choose the provider route' },
  { command: '/agents', description: 'inspect agent roles' },
  { command: '/files', description: 'search workspace files' },
  { command: '/git', description: 'inspect Git state' },
  { command: '/tasks', description: 'open task inspector' },
  { command: '/memory', description: 'search project memory' },
  { command: '/skills', description: 'list loaded skills' },
  { command: '/theme', description: 'switch terminal theme' },
  { command: '/settings', description: 'open UI settings' },
  { command: '/doctor', description: 'run local diagnostics' },
  { command: '/help', description: 'show keyboard help' }
];

const unique = (items: Message[]) => items.filter((item, index) => items.findIndex(other => other.id === item.id) === index);

export function App() {
  const { exit } = useApp();
  const { stdout } = useStdout();
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [draft, setDraft] = useState('');
  const [inputVersion, setInputVersion] = useState(0);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [commandPalette, setCommandPalette] = useState(false);
  const [selectedCommand, setSelectedCommand] = useState(0);
  const [skillManager, setSkillManager] = useState(false);
  const [skillManagerText, setSkillManagerText] = useState('Loading indexed skills…');
  const [skillManagerLoading, setSkillManagerLoading] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [terminalTick, setTerminalTick] = useState(0);

  const columns = Math.max(40, stdout.columns ?? process.stdout.columns ?? 120);
  const rows = Math.max(12, stdout.rows ?? process.stdout.rows ?? 36);
  const breakpoint = snapshot?.platform === 'termux' ? getTermuxBreakpoint(columns) : getBreakpoint(columns);
  const colorMode = getColorMode();
  const isMobile = breakpoint === 'mobile' || breakpoint === 'compact';
  const contentWidth = Math.max(28, columns - (isMobile ? 2 : 8));
  const showTips = !isMobile && rows >= 28;
  const showWarning = Boolean(snapshot && !snapshot.projectSpecific && !isMobile && rows >= 24);
  const skillManagerHeight = skillManager ? Math.min(22, 4 + skillManagerText.split(/\r?\n/).length) : 0;
  const chromeHeight = 3 + (isMobile ? 4 : 8) + (showTips ? 6 : 0) + (showWarning ? 3 : 0) + skillManagerHeight + 3 + 3;
  const chatHeight = Math.max(2, rows - chromeHeight);
  const visibleRows = Math.max(1, Math.floor(chatHeight / (isMobile ? 2 : 3)));
  const slashSuggestions = useMemo(() => {
    const query = draft.startsWith('/') ? draft.toLowerCase() : '';
    return query ? commands.filter(item => item.command.startsWith(query)).slice(0, 5) : [];
  }, [draft]);
  const contextSuggestions = useMemo(() => {
    const lastToken = draft.split(/\s+/).pop() ?? '';
    return lastToken.startsWith('@') ? ['@file', '@folder', '@url', '@agent', '@skill', '@memory'].filter(item => item.startsWith(lastToken)) : [];
  }, [draft]);
  const inputSuggestions = [...slashSuggestions.map(item => item.command), ...contextSuggestions];

  useEffect(() => {
    let active = true;
    let watcher: FSWatcher | undefined;
    const refresh = async () => {
      try {
        const next = await loadSnapshot();
        if (!active) return;
        setSnapshot(next);
        setMessages(current => current.length > 0 ? current : next.messages);
        setRuntimeError(null);
        watcher ??= watchRuntime(process.cwd(), refresh);
      } catch (error) {
        if (active) setRuntimeError(error instanceof Error ? error.message : 'Runtime unavailable');
      }
    };
    void refresh();
    return () => {
      active = false;
      void watcher?.close();
    };
  }, []);

  useEffect(() => {
    if (!streaming) return;
    const timer = setInterval(() => setTerminalTick(value => value + 1), 250);
    return () => clearInterval(timer);
  }, [streaming]);

  useEffect(() => {
    const resize = () => setTerminalTick(value => value + 1);
    process.on('SIGWINCH', resize);
    return () => { process.off('SIGWINCH', resize); };
  }, []);

  const openSkillManager = async (query?: string) => {
    setSkillManager(true);
    setSkillManagerLoading(true);
    const args = query ? ['search', query, '--limit', '12'] : ['list', '12'];
    const [output, categories] = await Promise.all([
      runSkillCommand(args),
      runSkillCommand(['categories'])
    ]);
    const categoryLine = categories.split(/\r?\n/).filter(Boolean).slice(0, 10).join(' · ');
    setSkillManagerText(`CATEGORIES · ${categoryLine}\n${output}`);
    setSkillManagerLoading(false);
  };

  useEffect(() => {
    const stdin = process.stdin;
    if (!stdin.isTTY) return;
    const onData = (chunk: Buffer | string) => {
      const value = chunk.toString();
      if (value.includes('\\u001b[<64;')) setScrollOffset(current => Math.min(messages.length, current + 2));
      if (value.includes('\\u001b[<65;')) setScrollOffset(current => Math.max(0, current - 2));
    };
    stdin.on('data', onData);
    return () => { stdin.off('data', onData); };
  }, [messages.length]);

  useInput((input, key) => {
    if (key.ctrl && input === 'c') {
      exit();
      return;
    }
    if (key.ctrl && input === 'k') {
      setCommandPalette(value => !value);
      return;
    }
    if (key.ctrl && input === 's') {
      void openSkillManager();
      return;
    }
    if (key.escape) {
      setCommandPalette(false);
      setSkillManager(false);
      return;
    }
    if (skillManager) return;
    if (commandPalette) {
      if (key.upArrow) setSelectedCommand(value => Math.max(0, value - 1));
      if (key.downArrow) setSelectedCommand(value => Math.min(commands.length - 1, value + 1));
      if (key.return) {
        setDraft(commands[selectedCommand]?.command ?? '/help');
        setInputVersion(value => value + 1);
        setCommandPalette(false);
      }
      return;
    }
    if (key.pageUp) setScrollOffset(value => Math.min(messages.length, value + Math.max(1, visibleRows - 2)));
    if (key.pageDown) setScrollOffset(value => Math.max(0, value - Math.max(1, visibleRows - 2)));
    if (key.upArrow && !draft) {
      const next = Math.min(history.length - 1, historyIndex + 1);
      if (next >= 0) {
        setHistoryIndex(next);
        setDraft(history[history.length - next - 1] ?? '');
        setInputVersion(value => value + 1);
      }
    }
    if (key.downArrow && !draft) {
      const next = Math.max(-1, historyIndex - 1);
      setHistoryIndex(next);
      setDraft(next === -1 ? '' : history[history.length - next - 1] ?? '');
      setInputVersion(value => value + 1);
    }
  }, { isActive: true });

  const addMessage = (message: Message) => setMessages(current => unique([...current, message]));

  const streamAssistant = async (prompt: string) => {
    setStreaming(true);
    const response = await submitPrompt(prompt);
    const responseId = `${Date.now()}-assistant`;
    const pendingTool = response.tool ? { ...response.tool, state: 'running' as const } : undefined;
    addMessage({ id: responseId, role: 'uthy', text: '', time: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }), tool: pendingTool });
    const tokens = response.text.split(/(\s+)/);
    let text = '';
    for (const token of tokens) {
      text += token;
      setMessages(current => current.map(message => message.id === responseId ? { ...message, text } : message));
      await new Promise(resolve => setTimeout(resolve, 18));
    }
    if (response.tool) {
      setMessages(current => current.map(message => message.id === responseId ? { ...message, tool: response.tool } : message));
    }
    setStreaming(false);
    setScrollOffset(0);
  };

  const handleSubmit = (value: string) => {
    const prompt = value.trim();
    if (!prompt || streaming) return;
    if (prompt === '/help') {
      setCommandPalette(true);
      return;
    }
    if (prompt === '/skills') {
      void openSkillManager();
      return;
    }
    if (prompt.startsWith('/skills search ')) {
      void openSkillManager(prompt.slice('/skills search '.length).trim());
      return;
    }
    setHistory(current => [...current.filter(item => item !== prompt), prompt].slice(-30));
    setHistoryIndex(-1);
    setDraft('');
    setInputVersion(value => value + 1);
    addMessage({ id: `${Date.now()}-user`, role: 'you', text: prompt, time: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }) });
    void streamAssistant(prompt);
  };

  const visibleMessages = messages.slice(Math.max(0, messages.length - visibleRows - scrollOffset), messages.length - scrollOffset || undefined);
  const placeholder = isMobile ? 'Ask UTHARNESS…' : 'Type your message or @path/to/file';

  return (
          <Box flexDirection="column" width="100%" height={rows} paddingX={isMobile ? 0 : 1}>

      <PersistentHeader breakpoint={breakpoint} colorMode={colorMode} platform={snapshot?.platform} />
      <Banner breakpoint={breakpoint} colorMode={colorMode} />
      {showTips ? <StartupTips colorMode={colorMode} termux={snapshot?.platform === 'termux'} /> : null}
      {showWarning ? <WorkspaceWarning colorMode={colorMode} /> : null}
      {skillManager ? <SkillManager text={skillManagerLoading ? 'Loading indexed skills…' : skillManagerText} width={contentWidth} colorMode={colorMode} /> : null}
      {runtimeError ? <Text color={colorMode === 'mono' ? undefined : palette.error}>Runtime: {runtimeError}</Text> : null}
      <Box flexDirection="column" height={chatHeight} overflow="hidden">
        {visibleMessages.map(message => <MessageRow key={message.id} message={message} width={contentWidth} colorMode={colorMode} />)}
        {streaming ? <Text color={colorMode === 'mono' ? undefined : palette.cyan}>  ◇ UTHY is streaming… {terminalTick % 2 === 0 ? '▌' : ' '}</Text> : null}
      </Box>
      {commandPalette ? (
        <Box borderStyle="round" borderColor={colorMode === 'mono' ? undefined : palette.purple} paddingX={1} flexDirection="column" width={Math.min(contentWidth, 72)}>
          <Text color={colorMode === 'mono' ? undefined : palette.purple} bold>COMMAND PALETTE · Ctrl+K</Text>
          {commands.map((command, index) => <Text key={command.command} color={index === selectedCommand ? (colorMode === 'mono' ? undefined : palette.cyan) : (colorMode === 'mono' ? undefined : palette.text)}>{index === selectedCommand ? '› ' : '  '}{command.command.padEnd(12)} {command.description}</Text>)}
          <Text color={colorMode === 'mono' ? undefined : palette.muted}>↑/↓ select · Enter insert · Esc close</Text>
        </Box>
      ) : null}
      <PromptFrame width={contentWidth} colorMode={colorMode}>
        <TextInput key={inputVersion} placeholder={placeholder} defaultValue={draft} suggestions={inputSuggestions} onChange={setDraft} onSubmit={handleSubmit} isDisabled={streaming || commandPalette || skillManager} />
      </PromptFrame>
      {slashSuggestions.length > 0 ? <Text color={colorMode === 'mono' ? undefined : palette.purple}>  {slashSuggestions.map(item => `${item.command} · ${item.description}`).join('   ')}</Text> : null}
      {snapshot ? <StatusBar snapshot={snapshot} width={contentWidth} colorMode={colorMode} /> : <Text color={colorMode === 'mono' ? undefined : palette.muted}>Loading runtime status…</Text>}
    </Box>
  );
}
