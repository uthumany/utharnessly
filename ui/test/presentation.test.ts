import test from 'node:test';
import assert from 'node:assert/strict';
import { asciiMode, featureFrames, icon, reducedMotion } from '../src/tui/icons.js';
import { divine, getColorMode } from '../src/tui/theme.js';

test('requested activity glyphs have printable ASCII alternatives', () => {
  assert.equal(icon('agent', true), '𓄆');
  assert.equal(icon('selector', true), '𓆃');
  assert.equal(icon('blinker', true), '𓇩');
  for (const name of ['agent', 'selector', 'blinker'] as const) assert.match(icon(name, false), /^[\x20-\x7e]+$/);
});
test('feature hieroglyphs map to bracketed ASCII fallbacks', () => {
  assert.equal(icon('computer', true), '𓂀');
  assert.equal(icon('improve', true), '𓋹');
  assert.equal(icon('memory', true), '𓏞');
  assert.equal(icon('build', true), '𓉐');
  assert.equal(icon('test', true), '𓄣');
  assert.equal(icon('fix', true), '𓌙');
  assert.equal(icon('computer', false), '[eye]');
  assert.equal(icon('improve', false), '[ankh]');
  assert.equal(icon('memory', false), '[scroll]');
  assert.equal(icon('fix', false), '[fix]');
  assert.ok(asciiMode({ NO_COLOR: '' }));
  assert.ok(asciiMode({ UTHARNESS_ASCII: '1' }));
  assert.ok(!asciiMode({ TERM: 'xterm-256color' }));
});

test('feature animation frames stay within the FPS budget and honor reduced motion', () => {
  for (const name of ['computer', 'improve', 'memory', 'build', 'test', 'fix']) {
    const frames = featureFrames(name);
    assert.ok(frames.length >= 2 && frames.length <= 10, `${name} has ${frames.length} frames`);
    assert.ok(frames.every(frame => frame.includes(icon(name as 'memory', true))), `${name} keeps its glyph`);
  }
  assert.deepEqual(featureFrames('memory', { UTHARNESS_REDUCED_MOTION: '1' }), ['𓏞']);
  assert.deepEqual(featureFrames('memory', { NO_COLOR: '' }), ['[scroll]']);
  assert.ok(reducedMotion({ UTHARNESS_REDUCED_MOTION: '1' }));
});

test('unified divine palette carries the five specified roles', () => {
  assert.equal(divine.gold, '#D4AF37');
  assert.equal(divine.lapis, '#1D3557');
  assert.equal(divine.hematite, '#E63946');
  assert.equal(divine.papyrus, '#F5E6D3');
  assert.equal(divine.nile, '#2A9D8F');
  assert.equal(divine.glyphWhite, '#FFFFFF');
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
