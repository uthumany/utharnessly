import test from 'node:test';
import assert from 'node:assert/strict';
import { icon } from '../src/tui/icons.js';
import { getColorMode } from '../src/tui/theme.js';

test('requested activity glyphs have printable ASCII alternatives', () => {
  assert.equal(icon('agent', true), '𓄆');
  assert.equal(icon('selector', true), '𓆃');
  assert.equal(icon('blinker', true), '𓇩');
  for (const name of ['agent', 'selector', 'blinker'] as const) assert.match(icon(name, false), /^[\x20-\x7e]+$/);
});
test('empty NO_COLOR and dumb terminals disable colors', () => {
  assert.equal(getColorMode({ NO_COLOR: '' }), 'mono');
  assert.equal(getColorMode({ TERM: 'dumb', COLORTERM: 'truecolor' }), 'mono');
});

test('badge presets remain readable without ANSI in monochrome mode', async () => {
  const implementation = await import('../src/tui/status.js').catch(() => null);
  assert.ok(implementation, 'Status badges need terminal capability fallbacks');
  for (const [kind, label] of [['success', 'PASS'], ['error', 'FAIL'], ['warning', 'WARN'], ['info', 'INFO'], ['neutral', 'N/A']] as const) {
    assert.equal(implementation.statusBadge(kind, label, 'mono'), `[${label}]`);
    assert.ok(implementation.statusBadge(kind, label, 'truecolor').includes(label));
  }
});
