import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Box, Text, useApp, useInput, useStdout } from 'ink';
import type { FSWatcher } from 'chokidar';
import { bannerHeight, Inspector, MessageRow, Navigation, Overlay, palette, PersistentHeader, StartupTips, StatusBar, WorkspaceWarning } from './components.js';
import { loadSnapshot, loadModelCatalog, runRuntimeCommand, submitAgent, submitChat, watchRuntime } from './runtime.js';
import { localCommandArgs, routePrompt } from './commands.js';
import { Composer } from './tui/composer.js';
import { transcriptPages } from './tui/transcript.js';
import { icon } from './tui/icons.js';
import { effectiveLayout, getBreakpoint, getTermuxBreakpoint, workspaceWidths } from './tui/responsive.js';
import { bannerTier } from './tui/banner.js';
import { defaultUiState, loadUiState, saveUiState } from './tui/state.js';
import { getColorMode, tone } from './tui/theme.js';
import type { Message, OverlayKind, PaletteItem, PersistedUiState, RuntimeSnapshot, ToolCard } from './types.js';

const commands: PaletteItem[] = [
  { id: 'help', label: '/help', description: 'keyboard and command help', shortcut: 'F1', overlay: 'help' },
  { id: 'new', label: '/new', description: 'start a new local session', command: '/new' },
  { id: 'agent', label: '/agent', description: 'run the bounded workspace agent on a task', command: '/agent' },
  { id: 'resume', label: '/resume', description: 'list saved sessions (CLI chat --session to resume)', command: '/resume' },
  { id: 'model', label: '/model', description: 'choose the active model', shortcut: 'Ctrl+P', overlay: 'models' },
  { id: 'provider', label: '/provider', description: 'inspect provider route', command: '/provider' },
  { id: 'agents', label: '/agents', description: 'inspect agent roles', shortcut: 'Ctrl+G', overlay: 'agents' },
  { id: 'tools', label: '/tools', description: 'inspect tool availability', command: '/tools' },
  { id: 'skills', label: '/skills', description: 'browse indexed skills', command: '/skills' },
  { id: 'memory', label: '/memory', description: 'search project memory', shortcut: 'Ctrl+M', overlay: 'memory' },
  { id: 'jobs', label: '/jobs', description: 'inspect background jobs', shortcut: 'Ctrl+J', overlay: 'jobs' },
  { id: 'files', label: '/files', description: 'browse workspace files', shortcut: 'Ctrl+O', overlay: 'files' },
  { id: 'git', label: '/git', description: 'inspect Git state', command: '/git' },
  { id: 'tasks', label: '/tasks', description: 'open task inspector', shortcut: 'Ctrl+T', overlay: 'tasks' },
  { id: 'status', label: '/status', description: 'refresh runtime telemetry', command: '/status' },
  { id: 'context', label: '/context', description: 'inspect known context', overlay: 'context' },
  { id: 'banner', label: '/banner', description: 'full, compact, or hide', command: '/banner' },
  { id: 'logs', label: '/logs', description: 'open runtime logs', shortcut: 'Ctrl+L', overlay: 'logs' },
  { id: 'doctor', label: '/doctor', description: 'run local diagnostics', command: '/doctor' },
  { id: 'quit', label: '/quit', description: 'exit Utharness', command: '/quit' }
];

const overlayDefaults: Record<Exclude<OverlayKind, null>, PaletteItem[]> = {
  commands,
  models: [],
  files: [{ id: 'cwd', label: '@file', description: 'type a path relative to this workspace' }, { id: 'folder', label: '@folder', description: 'reference a directory' }],
  agents: [{ id: 'safe', label: 'SAFE agent', description: 'Use /agents to inspect native capabilities' }],
  tasks: [{ id: 'idle', label: '○ Ready for input', description: 'No active task' }],
  memory: [{ id: 'search', label: 'Project memory', description: '/memory QUERY searches persisted memory' }],
  jobs: [{ id: 'unsupported', label: 'Background jobs unavailable', description: 'This release runs foreground tasks only' }],
  logs: [{ id: 'persisted', label: 'Persisted runtime events', description: 'Events are stored in the local SQLite database; no log browser yet' }],
  help: [
    { id: 'palette', label: 'Command palette', description: 'search all commands', shortcut: 'Ctrl+K' },
    { id: 'workspace', label: 'Workspace mode', description: 'toggle navigation and inspector', shortcut: 'Ctrl+B' },
    { id: 'file', label: 'File picker', description: 'open context files', shortcut: 'Ctrl+O' },
    { id: 'cancel', label: 'Cancel operation', description: 'cancel current task', shortcut: 'Ctrl+C' },
    { id: 'newline', label: 'Composer newline', description: 'insert a line break', shortcut: 'Shift+Enter' }
  ],
  context: [{ id: 'known', label: 'Runtime context', description: 'only known values are displayed in the status bar' }]
};

const now = () => new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
const unique = (items: Message[]) => items.filter((item, index) => items.findIndex(other => other.id === item.id) === index);

export function App() {
  const { exit } = useApp();
  const { stdout } = useStdout();
  const [, setResizeRevision] = useState(0);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const resized = () => { clearTimeout(timer); timer = setTimeout(() => setResizeRevision(value => value + 1), 50); };
    stdout.on('resize', resized);
    return () => { clearTimeout(timer); stdout.off('resize', resized); };
  }, [stdout]);
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot | null>(null);
  const [modelItems, setModelItems] = useState<PaletteItem[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [ui, setUi] = useState<PersistedUiState>(defaultUiState);
  const uiRef = useRef(ui);
  uiRef.current = ui;
  const [hydrated, setHydrated] = useState(false);
  const [overlay, setOverlay] = useState<OverlayKind>(null);
  const [overlayQuery, setOverlayQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [streaming, setStreaming] = useState(false);
  const activeRequest = useRef<AbortController | null>(null);
  useEffect(() => () => { activeRequest.current?.abort(); }, []);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const [composerFocused, setComposerFocused] = useState(true);

  const columns = Math.max(20, stdout.columns ?? process.stdout.columns ?? 120);
  const rows = Math.max(10, stdout.rows ?? process.stdout.rows ?? 36);
  const breakpoint = snapshot?.platform === 'termux' ? getTermuxBreakpoint(columns) : getBreakpoint(columns);
  const mode = effectiveLayout(ui.layoutMode, breakpoint);
  const colorMode = getColorMode();
  const compact = breakpoint === 'tiny' || breakpoint === 'compact';
  const contentWidth = Math.max(20, columns - (compact ? 0 : 4));
  const headerWidth = columns - (compact ? 0 : 2);
  const banner = bannerTier(headerWidth, rows, ui.bannerMode);
  const headerHeight = bannerHeight(banner);
  const hasOverlay = Boolean(overlay || ui.draft.startsWith('/') || (ui.draft.split(/\s+/).pop() ?? '').startsWith('@'));
  const showTips = !hasOverlay && !compact && rows >= 32;
  const showWarning = Boolean(!hasOverlay && snapshot && !snapshot.projectSpecific && !compact && rows >= 28);
  const showInputHints = columns >= 40 && rows >= 15;
  const footerHeight = 3 + (columns < 40 ? 1 : 3) + (showInputHints ? 1 : 0);
  const overlayLimit = Math.max(1, Math.min(12, rows - headerHeight - footerHeight - 7));
  const fixedHeight = 1 + headerHeight + (showTips ? 5 : 0) + (showWarning ? 4 : 0) + footerHeight + (hasOverlay ? overlayLimit + 5 : 0);
  const chatHeight = Math.max(1, rows - fixedHeight);
  const chatWidth = mode === 'workspace' ? workspaceWidths(columns).chat : contentWidth;
  const messageWidth = mode === 'workspace' ? chatWidth - 3 : chatWidth;
  const pages = transcriptPages(messages, messageWidth, Math.max(1, chatHeight - (streaming ? 1 : 0)));

  useEffect(() => { void loadUiState().then(state => { const bannerMode = ['full', 'compact', 'minimal', 'hide'].includes(process.env.UTHARNESS_BANNER ?? '') ? process.env.UTHARNESS_BANNER as PersistedUiState['bannerMode'] : state.bannerMode; const iconMode = ['nerd', 'unicode', 'ascii'].includes(process.env.UTHARNESS_ICONS ?? '') ? process.env.UTHARNESS_ICONS as PersistedUiState['iconMode'] : state.iconMode; setUi({ ...state, bannerMode, iconMode }); setHydrated(true); }); }, []);
  useEffect(() => { if (!hydrated) return; const timer = setTimeout(() => void saveUiState(ui).catch(() => undefined), 180); return () => clearTimeout(timer); }, [ui, hydrated]);
  useEffect(() => { if (!streaming || ui.reducedMotion) return; const timer = setInterval(() => setTick(value => value + 1), 500); return () => clearInterval(timer); }, [streaming, ui.reducedMotion]);

  useEffect(() => {
    let active = true;
    let watcher: FSWatcher | undefined;
    const refresh = async () => {
      try {
        const next = await loadSnapshot();
        if (!active) return;
        setSnapshot(current => ({ ...next, model: uiRef.current.selectedModel ?? current?.model ?? next.model, provider: uiRef.current.selectedProvider ?? current?.provider ?? next.provider }));
        setMessages(current => current.length ? current : next.messages);
        setRuntimeError(null);
        watcher ??= watchRuntime(process.cwd(), refresh);
      } catch (error) { if (active) setRuntimeError(error instanceof Error ? error.message : 'Runtime unavailable'); }
    };
    void refresh();
    return () => { active = false; void watcher?.close(); };
  }, []);

  const derivedOverlay = useMemo<OverlayKind>(() => {
    if (overlay) return overlay;
    const token = ui.draft.split(/\s+/).pop() ?? '';
    if (token.startsWith('@')) return 'files';
    if (ui.draft.startsWith('/')) return 'commands';
    return null;
  }, [overlay, ui.draft]);
  const allItems = derivedOverlay === 'models' ? modelItems : derivedOverlay ? overlayDefaults[derivedOverlay] : [];
  const query = overlay ? overlayQuery : (derivedOverlay === 'commands' ? ui.draft : (ui.draft.split(/\s+/).pop() ?? ''));
  const visibleItems = useMemo(() => {
    const needle = query.replace(/^[/@]/, '').toLowerCase();
    return allItems.filter(item => !needle || `${item.label} ${item.description}`.toLowerCase().includes(needle));
  }, [allItems, query]);

  const openOverlay = (kind: Exclude<OverlayKind, null>) => {
    setOverlay(kind); setOverlayQuery(''); setSelected(0); setComposerFocused(false);
    if (kind === 'models') {
      setModelItems([{ id: 'loading', label: 'Loading models…', description: 'Querying configured provider' }]);
      void loadModelCatalog(process.cwd(), ui.selectedProvider ?? snapshot?.provider).then(catalog => {
        setModelItems(catalog.models.map(model => ({ id: model, label: model, description: catalog.provider })));
      }).catch(error => setModelItems([{ id: 'error', label: 'Model catalog unavailable', description: String(error.message ?? error) }]));
    }
  };
  const closeOverlay = () => { setOverlay(null); setOverlayQuery(''); setSelected(0); setComposerFocused(true); };
  const setDraft = (draft: string) => setUi(current => ({ ...current, draft }));
  const reportError = (error: unknown) => setMessages(current => unique([...current, { id: `${Date.now()}-error`, role: 'error', text: error instanceof Error ? error.message : String(error), time: now() }]));
  const refreshSnapshot = async () => { try { const next = await loadSnapshot(); setSnapshot({ ...next, model: uiRef.current.selectedModel ?? next.model, provider: uiRef.current.selectedProvider ?? next.provider }); } catch (error) { reportError(error); } };

  const runLocalCommand = (prompt: string): boolean => {
    const [command, argument] = prompt.split(/\s+/, 2);
    if (command === '/quit') { activeRequest.current?.abort(); exit(); return true; }
    if (command === '/help') { openOverlay('help'); return true; }
    if (command === '/model') { openOverlay('models'); return true; }
    if (command === '/files') { openOverlay('files'); return true; }
    if (command === '/tasks') { openOverlay('tasks'); return true; }
    if (command === '/jobs') { openOverlay('jobs'); return true; }
    if (command === '/logs') { openOverlay('logs'); return true; }
    if (command === '/context') { openOverlay('context'); return true; }
    if (command === '/status') { void refreshSnapshot(); return true; }
    if (command === '/banner') { const next = argument === 'hide' || argument === 'minimal' || argument === 'compact' || argument === 'full' ? argument : 'full'; setUi(current => ({ ...current, bannerMode: next })); return true; }
    if (command === '/git') { void loadSnapshot().then(next => setMessages(current => unique([...current, { id: `${Date.now()}-git`, role: 'system', text: JSON.stringify(next.git, null, 2), time: now() }]))).catch(reportError); return true; }
    if (prompt.startsWith('/')) {
      try { void runRuntimeCommand(localCommandArgs(prompt)).then(text => { setMessages(current => unique([...current, { id: `${Date.now()}-command`, role: 'system', text: text || 'No results.', time: now() }])); setScrollOffset(0); }).catch(reportError); }
      catch (error) { reportError(error); }
      return true;
    }
    return false;
  };

  const send = (value: string) => {
    const prompt = value.trim();
    if (!prompt || streaming) return;
    let effective = prompt, useAgent = false;
    if (routePrompt(prompt) === 'agent') {
      effective = prompt.replace(/^\/agent\s+/, '');
      if (!effective) { reportError('Usage: /agent TASK — run the bounded workspace agent. Plain text chats.'); setDraft(''); return; }
      useAgent = true;
    } else if (runLocalCommand(prompt)) { setDraft(''); return; }
    setUi(current => ({ ...current, draft: '', history: [...current.history.filter(item => item !== prompt), prompt].slice(-50) }));
    setHistoryIndex(-1);
    const userMessage: Message = { id: `${Date.now()}-user`, role: 'you', text: prompt, time: now() };
    setMessages(current => unique([...current, userMessage]));
    setTick(0);
    setStreaming(true);
    const controller = new AbortController();
    activeRequest.current = controller;
    const backend: Promise<{ text: string; tool?: ToolCard }> = useAgent ? submitAgent(effective, process.cwd(), { provider: ui.selectedProvider ?? snapshot?.provider, model: ui.selectedModel ?? snapshot?.model, signal: controller.signal }) : submitChat(effective, process.cwd(), { provider: ui.selectedProvider ?? snapshot?.provider, model: ui.selectedModel ?? snapshot?.model, signal: controller.signal });
    void backend.then(response => {
      if (controller.signal.aborted) return;
      const id = `${Date.now()}-assistant`;
      setMessages(current => unique([...current, { id, role: 'utharness', text: response.text, time: now(), tool: response.tool }]));
      setScrollOffset(0);
    }).catch(error => setMessages(current => unique([...current, { id: `${Date.now()}-error`, role: controller.signal.aborted ? 'system' : 'error', text: error instanceof Error ? error.message : String(error), time: now() }]))).finally(() => { if (activeRequest.current === controller) { activeRequest.current = null; setStreaming(false); } });
  };

  const activateSelected = () => {
    const item = visibleItems[selected];
    if (!item) return;
    if (derivedOverlay === 'commands') { if (item.overlay) openOverlay(item.overlay); else setDraft(`${item.label} `); if (!item.overlay) closeOverlay(); return; }
    if (derivedOverlay === 'models') { if (item.id === 'loading' || item.id === 'error') return; setUi(current => ({ ...current, selectedModel: item.id, selectedProvider: item.description })); setSnapshot(current => current ? { ...current, model: item.id, provider: item.description } : current); closeOverlay(); return; }
    if (derivedOverlay === 'files') { const token = ui.draft.split(/\s+/).pop() ?? ''; setDraft(`${ui.draft.slice(0, ui.draft.length - token.length)}${item.label} `); closeOverlay(); return; }
    closeOverlay();
  };

  useInput((input, key) => {
    if (key.ctrl && input === 'c') { if (streaming) activeRequest.current?.abort(); else exit(); return; }
    if (key.ctrl && input === 'b') { setUi(current => ({ ...current, layoutMode: current.layoutMode === 'focus' ? 'workspace' : 'focus' })); return; }
    const shortcuts: Record<string, Exclude<OverlayKind, null>> = { k: 'commands', p: 'models', o: 'files', g: 'agents', t: 'tasks', m: 'memory', j: 'jobs', l: 'logs' };
    if (key.ctrl && shortcuts[input]) { openOverlay(shortcuts[input]!); return; }
    if (key.tab) { setComposerFocused(value => !value); return; }
    if (key.escape) { if (overlay) closeOverlay(); else setComposerFocused(value => !value); return; }
    if (input === '\u001bOP') { openOverlay('help'); return; }
    if (overlay) {
      if (key.upArrow) setSelected(value => Math.max(0, value - 1));
      else if (key.downArrow) setSelected(value => Math.min(Math.max(0, visibleItems.length - 1), value + 1));
      else if (key.return) activateSelected();
      else if (key.backspace || key.delete) setOverlayQuery(value => value.slice(0, -1));
      else if (input && !key.ctrl) setOverlayQuery(value => value + input);
      return;
    }
    if (key.pageUp) setScrollOffset(value => Math.min(Math.max(0, pages.length - 1), value + 1));
    if (key.pageDown) setScrollOffset(value => Math.max(0, value - 1));
    if (key.upArrow && composerFocused && !ui.draft && ui.history.length) { const next = Math.min(ui.history.length - 1, historyIndex + 1); setHistoryIndex(next); setDraft(ui.history[ui.history.length - next - 1] ?? ''); }
    if (key.downArrow && composerFocused && historyIndex >= 0) { const next = historyIndex - 1; setHistoryIndex(next); setDraft(next < 0 ? '' : (ui.history[ui.history.length - next - 1] ?? '')); }
  });

  const pageIndex = Math.max(0, pages.length - 1 - scrollOffset);
  const visibleMessages = pages.slice(pageIndex, pageIndex + 1);
  const chat = <Box flexDirection="column" width={chatWidth} height={chatHeight} overflow="hidden" paddingX={mode === 'workspace' ? 1 : 0}>{runtimeError ? <Text color={tone(palette.error, colorMode)}>Runtime: {runtimeError}</Text> : null}{visibleMessages.map(message => <MessageRow key={message.id} message={message} width={mode === 'workspace' ? chatWidth - 3 : chatWidth} colorMode={colorMode} tick={tick} />)}{streaming ? <Text color={tone(palette.primary, colorMode)} wrap="truncate-end"><Text color={tone(palette.error, colorMode)} dimColor={!ui.reducedMotion && tick % 2 === 1}>{icon('blinker', ui.iconMode !== 'ascii')}</Text> Agent working · Ctrl+C cancels</Text> : null}</Box>;

  return <Box flexDirection="column" width={columns} height={rows - 1} paddingX={compact ? 0 : 1}>
    <Box flexShrink={0} height={headerHeight}><PersistentHeader width={headerWidth} rows={rows} mode={ui.bannerMode} colorMode={colorMode} iconMode={ui.iconMode} /></Box>
    {showTips ? <StartupTips colorMode={colorMode} /> : null}
    {showWarning ? <WorkspaceWarning colorMode={colorMode} /> : null}
    {mode === 'workspace' && snapshot ? <Box height={chatHeight}><Navigation colorMode={colorMode} width={workspaceWidths(columns).navigation} />{chat}<Inspector snapshot={snapshot} colorMode={colorMode} width={workspaceWidths(columns).inspector} /></Box> : chat}
    {derivedOverlay ? <Overlay kind={derivedOverlay} items={visibleItems} selected={selected} query={query} width={contentWidth} colorMode={colorMode} maxItems={overlayLimit} /> : null}
    <Composer value={ui.draft} onChange={setDraft} onSubmit={send} width={contentWidth} colorMode={colorMode} focused={composerFocused && !overlay} disabled={streaming || Boolean(overlay)} placeholder={compact ? 'Ask Utharness…' : 'Type your message or @path/to/file'} />
    {showInputHints ? <Text color={tone(palette.muted, colorMode)} wrap="truncate-end"> Enter send · PgUp/PgDn results {pages.length ? `${pageIndex + 1}/${pages.length}` : ''} · Ctrl+K commands</Text> : null}
    {snapshot ? <StatusBar snapshot={snapshot} width={contentWidth} colorMode={colorMode} /> : <Text color={tone(palette.muted, colorMode)}>Loading runtime status…</Text>}
  </Box>;
}
