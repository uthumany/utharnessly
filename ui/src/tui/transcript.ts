import wrapAnsi from 'wrap-ansi';
import type { Message } from '../types.js';

/** Page by rendered rows, not message count, so long tool results remain reachable. */
export function transcriptPages(messages: Message[], width: number, height: number): Message[] {
  const bodyRows = Math.max(1, height - 3);
  return messages.flatMap(message => {
    const tool = message.tool;
    const text = tool ? `${message.text}\n${tool.name} · ${tool.state}\n${tool.detail ?? tool.result}\n${tool.metric} ${tool.elapsed}` : message.text;
    const lines = wrapAnsi(text, Math.max(1, width - 5), { hard: true }).split('\n');
    const pages: Message[] = [];
    for (let start = 0; start < lines.length;) {
      // Fill the newest page, placing any short remainder on the first page.
      const count = start === 0 ? (lines.length % bodyRows || bodyRows) : bodyRows;
      pages.push({ ...message, id: `${message.id}-page-${start}`, text: lines.slice(start, start + count).join('\n'), tool: undefined });
      start += count;
    }
    return pages;
  });
}
