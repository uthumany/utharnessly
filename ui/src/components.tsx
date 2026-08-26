import React from 'react';
import { Box, Text } from 'ink';
import { Spinner } from '@inkjs/ui';
import figures from 'figures';
import gradient from 'gradient-string';
import stringWidth from 'string-width';
import wrapAnsi from 'wrap-ansi';
import type { Breakpoint, ColorMode, Message, RuntimeSnapshot, ToolCard } from './types.js';

export const palette = {
  background: '#070B14',
  surface: '#0D1420',
  border: '#303948',
  text: '#D8DCE5',
  muted: '#70798A',
  amber: '#F6B817',
  orange: '#FF963F',
  coral: '#FF5F63',
  cyan: '#27D3C5',
  purple: '#D06BFF',
  green: '#7BD950',
  error: '#FF5C68'
} as const;

const brandGradient = gradient(['#F6B817', '#FF963F', '#FF5F63']);

export function getBreakpoint(columns: number): Breakpoint {
  if (columns >= 200) return 'ultra';
  if (columns >= 120) return 'wide';
  if (columns >= 80) return 'standard';
  if (columns >= 60) return 'narrow';
  return 'compact';
}

export function getColorMode(): ColorMode {
  if (process.env.NO_COLOR || process.env.UTHARNESS_ASCII === '1') return 'mono';
  if (process.env.UTHARNESS_COLOR === 'truecolor' || process.env.COLORTERM?.includes('truecolor')) return 'truecolor';
  if (process.env.UTHARNESS_COLOR === 'ansi256' || process.env.TERM?.includes('256color')) return 'ansi256';
  return 'ansi16';
}

function tone(hex: string, mode: ColorMode): string | undefined {
  return mode === 'mono' ? undefined : hex;
}

const wideAscii = [
  '██╗   ██╗████████╗██╗  ██╗ █████╗ ██████╗ ███╗   ██╗███████╗███████╗',
  '██║   ██║╚══██╔══╝██║  ██║██╔══██╗██╔══██╗████╗  ██║██╔════╝██╔════╝',
  '██║   ██║   ██║   ███████║███████║██████╔╝██╔██╗ ██║█████╗  ███████╗',
  '██║   ██║   ██║   ██╔══██║██╔══██║██╔══██╗██║╚██╗██║██╔══╝  ╚════██║',
  '╚██████╔╝   ██║   ██║  ██║██║  ██║██║  ██║██║ ╚████║███████╗███████║',
  ' ╚═════╝    ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝'
];

const mediumAscii = [
  '██╗   ██╗████████╗██╗  ██╗ █████╗ ██████╗',
  '██║   ██║╚══██╔══╝██║  ██║██╔══██╗██╔══██╗',
  '██║   ██║   ██║   ███████║███████║██████╔╝',
  '██║   ██║   ██║   ██╔══██║██╔══██║██╔══██╗',
  '╚██████╔╝   ██║   ██║  ██║██║  ██║██║  ██║',
  ' ╚═════╝    ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝'
];

const compactAscii = ['UTHARNESS', 'AGENT TERMINAL'];

export function Banner({ breakpoint, colorMode }: { breakpoint: Breakpoint; colorMode: ColorMode }) {
  const lines = breakpoint === 'compact' ? compactAscii : breakpoint === 'narrow' ? ['UTHARNESS', 'AGENT TERMINAL'] : breakpoint === 'standard' ? mediumAscii : wideAscii;
  return (
    <Box flexDirection="column" marginTop={1} marginBottom={1}>
      {breakpoint === 'compact' ? (
        <Text color={colorMode === 'mono' ? undefined : palette.amber}>{['UTHARNESS', 'AGENT TERMINAL'].join(String.fromCharCode(10))}</Text>
      ) : lines.map((line, index) => (
        <Text key={`${line}-${index}`}>
          {colorMode === 'mono' ? line : brandGradient(line)}
        </Text>
      ))}
    </Box>
  );
}

export function PersistentHeader({ breakpoint, colorMode }: { breakpoint: Breakpoint; colorMode: ColorMode }) {
  const muted = tone(palette.muted, colorMode);
  return (
    <Box width="100%" justifyContent="space-between" borderStyle="single" borderColor={tone(palette.border, colorMode)} paddingX={1}>
      <Text color={tone(palette.text, colorMode)} bold>{breakpoint === 'compact' ? 'UTHARNESS · focus mode' : 'UTHARNESS AGENT — focus mode'}</Text>
      {breakpoint === 'compact' ? <Text color={muted}>⌘K · ?</Text> : <Text color={muted}>⌘K commands    ? help</Text>}
    </Box>
  );
}

export function StartupTips({ colorMode }: { colorMode: ColorMode }) {
  return (
    <Box flexDirection="column" marginBottom={1}>
      <Text color={tone(palette.amber, colorMode)} bold>Tips for getting started:</Text>
      <Text color={tone(palette.text, colorMode)}>1. Ask questions, edit files, or run commands.</Text>
      <Text color={tone(palette.text, colorMode)}>2. Be specific for the best results.</Text>
      <Text color={tone(palette.text, colorMode)}>3. Use <Text color={tone(palette.cyan, colorMode)}>@path/to/file</Text> to add context.</Text>
      <Text color={tone(palette.text, colorMode)}>4. Type <Text color={tone(palette.purple, colorMode)}>/help</Text> for more information.</Text>
    </Box>
  );
}

export function WorkspaceWarning({ colorMode }: { colorMode: ColorMode }) {
  return (
    <Box borderStyle="round" borderColor={tone(palette.amber, colorMode)} paddingX={1} marginBottom={1}>
      <Text color={tone(palette.amber, colorMode)}>{figures.warning}  You are not in a project-specific directory.\n   For best results, open a workspace folder with your code and run utharness there.</Text>
    </Box>
  );
}

function toolColor(tool: ToolCard, colorMode: ColorMode): string | undefined {
  if (tool.state === 'error') return tone(palette.error, colorMode);
  if (tool.state === 'approval') return tone(palette.amber, colorMode);
  if (tool.state === 'running') return tone(palette.cyan, colorMode);
  return tone(palette.green, colorMode);
}

export function ToolCardView({ tool, width, colorMode }: { tool: ToolCard; width: number; colorMode: ColorMode }) {
  const cardWidth = Math.max(28, Math.min(78, width - 8));
  const color = toolColor(tool, colorMode);
  const status = tool.state === 'running' ? <Spinner label="Running" type="dots" /> : <Text color={color}>{tool.state === 'error' ? 'Error' : tool.state === 'approval' ? 'Approval required' : 'Completed'}</Text>;
  if (width < 100) {
    const compact = `${tool.icon} ${tool.name}  ${tool.state === 'running' ? 'Running' : tool.state === 'error' ? 'Error' : tool.state === 'approval' ? 'Approval' : 'Completed'}  ${tool.metric}  ${tool.elapsed}`;
    return (
      <Box width={cardWidth} borderStyle="round" borderColor={tone(palette.border, colorMode)} paddingX={1} marginTop={1}>
        <Text color={toolColor(tool, colorMode)} wrap="truncate-end">{compact}</Text>
      </Box>
    );
  }
  return (
    <Box flexDirection="column" width={cardWidth} borderStyle="round" borderColor={tone(palette.border, colorMode)} paddingX={1} marginTop={1}>
      <Box flexDirection="row" width={Math.max(10, cardWidth - 2)}>
        <Box width={Math.max(12, Math.floor((cardWidth - 2) * 0.38))}>
          <Text color={tone(palette.purple, colorMode)} wrap="truncate-end">{tool.icon} {tool.name}</Text>
        </Box>
        <Box width={Math.max(12, Math.floor((cardWidth - 2) * 0.28))}>
          <Text color={toolColor(tool, colorMode)} wrap="truncate-end">{status}</Text>
        </Box>
        <Box flexGrow={1} justifyContent="flex-end">
          <Text color={tone(palette.text, colorMode)} wrap="truncate-end">{tool.metric}  {tool.elapsed}</Text>
        </Box>
      </Box>
    </Box>
  );
}

export function MessageRow({ message, width, colorMode }: { message: Message; width: number; colorMode: ColorMode }) {
  const isUthy = message.role === 'uthy';
  const avatarColor = isUthy ? palette.purple : palette.cyan;
  const name = isUthy ? 'UTHY' : 'YOU';
  const messageWidth = Math.max(12, width - 6);
  const messageText = stringWidth(message.text) > messageWidth ? wrapAnsi(message.text, messageWidth, { hard: true }) : message.text;
  return (
    <Box flexDirection="column" marginBottom={1} width={Math.max(20, width)}>
      <Text>
        <Text color={tone(avatarColor, colorMode)} bold>{isUthy ? '◉' : '◌'}  {name}</Text>
        {width >= 60 ? <Text color={tone(palette.muted, colorMode)}>{' '.repeat(Math.max(1, width - 16 - message.time.length))}{message.time}</Text> : null}
        {String.fromCharCode(10)}{' '.repeat(4)}<Text color={tone(palette.text, colorMode)}>{messageText}</Text>
      </Text>
      {message.tool ? <Box paddingLeft={4}><ToolCardView tool={message.tool} width={width} colorMode={colorMode} /></Box> : null}
      {message.tool?.state === 'completed' && message.text.startsWith('Here are') ? <Box paddingLeft={5}><Text color={tone(palette.green, colorMode)}>{figures.arrowRight} results displayed to you</Text></Box> : null}
    </Box>
  );
}

export function PromptFrame({ width, colorMode, children }: { width: number; colorMode: ColorMode; children: React.ReactNode }) {
  return (
    <Box borderStyle="round" borderColor={tone(palette.cyan, colorMode)} paddingX={1} width={Math.max(24, width)} marginTop={1}>
      <Text color={tone(palette.cyan, colorMode)}>{figures.pointer} </Text>
      {children}
    </Box>
  );
}

export function StatusBar({ snapshot, width, colorMode }: { snapshot: RuntimeSnapshot; width: number; colorMode: ColorMode }) {
  if (width < 100) {
    const compact = `${snapshot.workspace}  │  ${snapshot.permission}  │  ${snapshot.provider}/${snapshot.model}  │  ${snapshot.branch}  │  ${snapshot.context}`;
    return (
      <Box borderStyle="single" borderColor={tone(palette.border, colorMode)} paddingX={1} marginTop={1} width={Math.max(24, width)}>
        <Text color={tone(palette.cyan, colorMode)} wrap="truncate-end">{compact}</Text>
      </Box>
    );
  }
  const segments = [
    ['▣', snapshot.workspace, palette.cyan],
    ['◈', snapshot.permission, palette.amber],
    ['✦', `${snapshot.provider}/${snapshot.model}`, palette.purple],
    ['⌘', snapshot.branch, palette.green],
    ['≡', snapshot.context, palette.cyan]
  ];
  return (
    <Box borderStyle="single" borderColor={tone(palette.border, colorMode)} paddingX={1} marginTop={1} width={Math.max(24, width)}>
      {segments.map(([icon, value, color], index) => (
        <React.Fragment key={`${icon}-${value}`}>
          <Text color={tone(color as string, colorMode)}>{icon} {value}</Text>
          {index < segments.length - 1 ? <Text color={tone(palette.muted, colorMode)}>  │  </Text> : null}
        </React.Fragment>
      ))}
      {width >= 120 ? <><Text color={tone(palette.muted, colorMode)}>  │  </Text><Text color={tone(snapshot.network === 'connected' ? palette.green : palette.muted, colorMode)}>{snapshot.network}</Text></> : null}
    </Box>
  );
}
