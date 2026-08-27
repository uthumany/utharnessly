import test from 'node:test';
import assert from 'node:assert/strict';
import { getBreakpoint, getColorMode, getTermuxBreakpoint } from '../src/components.js';
import { loadSnapshot } from '../src/runtime.js';

test('maps requested terminal width breakpoints', () => {
  assert.equal(getBreakpoint(40), 'compact');
  assert.equal(getBreakpoint(60), 'narrow');
  assert.equal(getBreakpoint(80), 'standard');
  assert.equal(getBreakpoint(120), 'wide');
  assert.equal(getBreakpoint(200), 'ultra');
});

test('maps Termux mobile-first width tiers', () => {
  assert.equal(getTermuxBreakpoint(40), 'mobile');
  assert.equal(getTermuxBreakpoint(50), 'narrow');
  assert.equal(getTermuxBreakpoint(89), 'narrow');
  assert.equal(getTermuxBreakpoint(90), 'standard');
});

test('respects limited-color environment fallback', () => {
  const original = process.env.NO_COLOR;
  process.env.NO_COLOR = '1';
  assert.equal(getColorMode(), 'mono');
  if (original === undefined) delete process.env.NO_COLOR;
  else process.env.NO_COLOR = original;
});

test('detects Termux runtime metadata without requiring Android commands', async () => {
  const previousTermux = process.env.TERMUX_VERSION;
  const previousPrefix = process.env.PREFIX;
  process.env.TERMUX_VERSION = '0.118.0';
  process.env.PREFIX = '/data/data/com.termux/files/usr';
  const snapshot = await loadSnapshot(process.cwd());
  assert.equal(snapshot.platform, 'termux');
  assert.equal(snapshot.prefix, '/data/data/com.termux/files/usr');
  if (previousTermux === undefined) delete process.env.TERMUX_VERSION;
  else process.env.TERMUX_VERSION = previousTermux;
  if (previousPrefix === undefined) delete process.env.PREFIX;
  else process.env.PREFIX = previousPrefix;
});

test('loads a zod-validated runtime snapshot', async () => {
  const snapshot = await loadSnapshot(process.cwd());
  assert.ok(snapshot.workspace.length > 0);
  assert.ok(snapshot.messages.length >= 4);
  assert.ok(['offline', 'openrouter'].includes(snapshot.provider));
  assert.ok(snapshot.platform.length > 0);
  assert.ok(snapshot.termuxApi.length > 0);
  assert.ok(snapshot.storage.length > 0);
});
