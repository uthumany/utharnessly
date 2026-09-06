import test from 'node:test';
import assert from 'node:assert/strict';
import { transcriptPages } from '../src/tui/transcript.js';

test('long tool output is reachable in bounded pages on short terminals', () => {
  const text = Array.from({ length: 40 }, (_, i) => `file-${i}.rs`).join('\n');
  const pages = transcriptPages([{ id: 'result', role: 'utharness', text, time: '12:00' }], 40, 8);
  assert.equal(pages.length, 8);
  assert.equal(pages.map(page => page.text).join('\n'), text);
  assert.ok(pages.every(page => page.text.split('\n').length <= 5));
  assert.match(pages.at(-1)!.text, /file-39.rs/);
});

test('wrapped long lines and narrow terminals keep all content', () => {
  const text = 'x'.repeat(100);
  const pages = transcriptPages([{ id: 'result', role: 'utharness', text, time: '' }], 20, 5);
  assert.equal(pages.map(page => page.text).join('').replaceAll('\n', ''), text);
  assert.ok(pages.every(page => page.text.split('\n').every(line => line.length <= 15)));
  assert.equal(pages.at(-1)!.text.split('\n').length, 2);
});
