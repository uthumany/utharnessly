/** Only explicitly supported commands may cross the native CLI boundary. */
export function localCommandArgs(prompt: string): string[] {
  const [command = '', ...rest] = prompt.trim().split(/\s+/);
  const argument = rest.join(' ');
  switch (command) {
    case '/new': return ['sessions', 'new', argument || 'TUI session'];
    case '/resume': return ['sessions', 'list'];
    case '/memory': return argument ? ['memory', 'search', argument] : ['memory', 'list'];
    case '/provider': return ['providers', 'list'];
    case '/agents': return ['agents', 'list'];
    case '/tools': return ['tools'];
    case '/doctor': return ['doctor'];
    case '/skills': return argument ? ['skills', 'search', argument] : ['skills', 'list'];
    case '/mcp': return ['mcp'];
    case '/checkpoint': return ['checkpoint'];
    default: throw new Error(`Unknown command: ${command}. Use /help or Ctrl+K.`);
  }
}
