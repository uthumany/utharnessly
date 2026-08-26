export type Breakpoint = 'compact' | 'narrow' | 'standard' | 'wide' | 'ultra';

export type ColorMode = 'truecolor' | 'ansi256' | 'ansi16' | 'mono';

export type Role = 'uthy' | 'you';
export type ToolState = 'running' | 'completed' | 'error' | 'approval';

export type ToolCard = {
  id: string;
  name: string;
  icon: string;
  state: ToolState;
  result: string;
  metric: string;
  elapsed: string;
};

export type Message = {
  id: string;
  role: Role;
  text: string;
  time: string;
  tool?: ToolCard;
};

export type RuntimeSnapshot = {
  workspace: string;
  permission: string;
  provider: string;
  model: string;
  branch: string;
  context: string;
  network: string;
  projectSpecific: boolean;
  messages: Message[];
};

export type CommandItem = {
  command: string;
  description: string;
};

export type UiState = {
  draft: string;
  history: string[];
  historyIndex: number;
  scrollOffset: number;
  commandPalette: boolean;
  slashSuggestions: CommandItem[];
  selectedCommand: number;
  streaming: boolean;
  reducedMotion: boolean;
};
