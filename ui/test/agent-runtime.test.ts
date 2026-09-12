import test from 'node:test';
import assert from 'node:assert/strict';
import * as runtime from '../src/runtime.js';
const { submitPrompt } = runtime;
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

test('selected provider and model reach the backend process', { skip: process.platform === 'win32' }, async () => {
  const original = process.env.UTHARNESS_RUNTIME_BIN;
  process.env.UTHARNESS_RUNTIME_BIN = fileURLToPath(new URL('./fixtures/agent-runtime.sh', import.meta.url));
  try {
    const response = await submitPrompt('route', process.cwd(), { provider: 'groq', model: 'openai/gpt-oss-20b' });
    assert.equal(response.text, 'groq/openai/gpt-oss-20b');
  } finally {
    if (original === undefined) delete process.env.UTHARNESS_RUNTIME_BIN;
    else process.env.UTHARNESS_RUNTIME_BIN = original;
  }
});

test('aborting a task terminates the backend and reports cancellation', { skip: process.platform === 'win32' }, async () => {
  const original = process.env.UTHARNESS_RUNTIME_BIN;
  process.env.UTHARNESS_RUNTIME_BIN = fileURLToPath(new URL('./fixtures/agent-runtime.sh', import.meta.url));
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 100);
  try {
    await assert.rejects(submitPrompt('wait', process.cwd(), { signal: controller.signal }), /cancel/i);
  } finally {
    clearTimeout(timer);
    if (original === undefined) delete process.env.UTHARNESS_RUNTIME_BIN;
    else process.env.UTHARNESS_RUNTIME_BIN = original;
  }
});

test('model picker loads provider models instead of fixed OpenAI choices', { skip: process.platform === 'win32' }, async () => {
  assert.equal(typeof runtime.loadModelCatalog, 'function');
  const original = process.env.UTHARNESS_RUNTIME_BIN;
  process.env.UTHARNESS_RUNTIME_BIN = fileURLToPath(new URL('./fixtures/agent-runtime.sh', import.meta.url));
  try {
    const result = await runtime.loadModelCatalog(process.cwd(), 'groq');
    assert.deepEqual(result.models, ['model-a', 'model-b']);
    assert.equal(result.provider, 'groq');
  } finally {
    if (original === undefined) delete process.env.UTHARNESS_RUNTIME_BIN;
    else process.env.UTHARNESS_RUNTIME_BIN = original;
  }
});
