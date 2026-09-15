import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));

test('launcher --version is sourced from package metadata', async () => {
  const manifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
  const output = execFileSync(process.execPath, ['bin/utharnessly.js', '--version'], {
    cwd: root,
    encoding: 'utf8',
  }).trim();

  assert.equal(output, `utharnessly ${manifest.version}`);
});
