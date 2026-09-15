export type IconName = 'selector' | 'blinker' | 'chat' | 'task' | 'running' | 'waiting' | 'failed' | 'warning' | 'success' | 'file' | 'folder' | 'agent' | 'you' | 'skill' | 'memory' | 'job' | 'model' | 'tool' | 'logs' | 'settings' | 'git' | 'shell' | 'search' | 'context' | 'edit' | 'test' | 'build' | 'lock' | 'computer' | 'improve' | 'fix';
const unicode: Record<IconName, string> = { selector: '𓆃', blinker: '𓇩', chat: '◇', task: '✓', running: '◆', waiting: '○', failed: '✗', warning: '!', success: '✓', file: '▤', folder: '▣', agent: '𓁬', you: '𓁶', skill: '✦', memory: '𓏞', job: '◷', model: '◇', tool: '⚙', logs: '≡', settings: '⚙', git: '⑂', shell: '>', search: '⌕', context: '▥', edit: '✎', test: '𓄣', build: '𓉐', lock: '◈', computer: '𓂀', improve: '𓋹', fix: '𓌙' };
const ascii: Record<IconName, string> = { selector: '>', blinker: '*', chat: '[*]', task: '[+]', running: '[*]', waiting: '[-]', failed: '[x]', warning: '[!]', success: '[+]', file: '[file]', folder: '[dir]', agent: '[agent]', you: '[you]', skill: '[skill]', memory: '[scroll]', job: '[job]', model: '[model]', tool: '[tool]', logs: '[log]', settings: '[cfg]', git: '[git]', shell: '[>]', search: '[?]', context: '[ctx]', edit: '[edit]', test: '[test]', build: '[build]', lock: '[lock]', computer: '[eye]', improve: '[ankh]', fix: '[fix]' };
export function asciiMode(env: NodeJS.ProcessEnv = process.env): boolean {
  return env.NO_COLOR !== undefined || env.TERM === 'dumb' || env.UTHARNESS_ASCII === '1' || env.UTHARNESS_ICONS === 'ascii';
}
export function icon(name: IconName, useUnicode = !asciiMode()): string { return (useUnicode ? unicode : ascii)[name]; }
export const spinnerFrames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
export function reducedMotion(env: NodeJS.ProcessEnv = process.env): boolean {
  return env.UTHARNESS_REDUCED_MOTION === '1' || env.NO_COLOR !== undefined;
}
// Feature animation cycles (≤12 FPS, ≤3s loops). Each frame pairs the feature
// glyph with a braille phase: blink for the eye, pulse for the ankh, unroll
// for the scroll, stack for the plan, balance for the heart, rotation for the adze.
const featureCycles: Record<string, string[]> = {
  computer: ['𓂀⠋', '𓂀⠹', '𓂀⠼', '𓂀⠦', '𓂀⠇', '𓂀⠏'],
  improve: ['𓋹⠋', '𓋹⠸', '𓋹⠴', '𓋹⠧'],
  memory: ['𓏞⠋', '𓏞⠹', '𓏞⠼', '𓏞⠦'],
  build: ['𓉐⠋', '𓉐⠸', '𓉐⠴', '𓉐⠧'],
  test: ['𓄣⠋', '𓄣⠹', '𓄣⠼', '𓄣⠦'],
  fix: ['𓌙⠋', '𓌙⠙', '𓌙⠹', '𓌙⠸', '𓌙⠼', '𓌙⠴', '𓌙⠦', '𓌙⠧', '𓌙⠇', '𓌙⠏']
};
export function featureFrames(name: string, env: NodeJS.ProcessEnv = process.env): string[] {
  if (reducedMotion(env) || asciiMode(env)) return [icon(name as IconName, !asciiMode(env))];
  return featureCycles[name] ?? spinnerFrames;
}
