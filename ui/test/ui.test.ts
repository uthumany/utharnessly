import test from 'node:test';
import assert from 'node:assert/strict';
import { getBreakpoint, getColorMode } from '../src/components.js';
import { loadSnapshot } from '../src/runtime.js';

test('maps requested terminal width breakpoints', () => {
  assert.equal(getBreakpoint(40), 'compact');
  assert.equal(getBreakpoint(60), 'narrow');
  assert.equal(getBreakpoint(80), 'standard');
  assert.equal(getBreakpoint(120), 'wide');
  assert.equal(getBreakpoint(200), 'ultra');
});

test('respects limited-color environment fallback', () => {
  const original = process.env.NO_COLOR;
  process.env.NO_COLOR = '1';
  assert.equal(getColorMode(), 'mono');
  if (original === undefined) delete process.env.NO_COLOR;
  else process.env.NO_COLOR = original;
});

test('loads a zod-validated runtime snapshot', async () => {
  const snapshot = await loadSnapshot(process.cwd());
  assert.ok(snapshot.workspace.length > 0);
  assert.ok(snapshot.messages.length >= 4);
  assert.ok(['offline', 'openrouter'].includes(snapshot.provider));
});
