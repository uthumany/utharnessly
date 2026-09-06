import test from 'node:test';
import assert from 'node:assert/strict';
import { submitPrompt } from '../src/runtime.js';
import fs from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

test('missing runtime rejects rather than inventing a completed response', async () => {
  const original = process.env.UTHARNESS_RUNTIME_BIN;
  process.env.UTHARNESS_RUNTIME_BIN = '/nonexistent/utharness-test-runtime';
  try {
    await assert.rejects(submitPrompt('List workspace files'), /runtime/i);
  } finally {
    if (original === undefined) delete process.env.UTHARNESS_RUNTIME_BIN;
    else process.env.UTHARNESS_RUNTIME_BIN = original;
  }
});

test('workspace tasks use the agent and nonzero exits surface provider errors', { skip: process.platform === 'win32' }, async () => {
  const original = process.env.UTHARNESS_RUNTIME_BIN;
  const fixture = fileURLToPath(new URL('./fixtures/agent-runtime.sh', import.meta.url));
  await fs.chmod(fixture, 0o755);
  process.env.UTHARNESS_RUNTIME_BIN = fixture;
  try {
    const response = await submitPrompt('List workspace files');
    assert.match(response.text, /README.md\n   Cargo.toml/);
    assert.equal(response.tool, undefined);
    await assert.rejects(submitPrompt('fail'), /HTTP 401/);
  } finally {
    if (original === undefined) delete process.env.UTHARNESS_RUNTIME_BIN;
    else process.env.UTHARNESS_RUNTIME_BIN = original;
  }
});
