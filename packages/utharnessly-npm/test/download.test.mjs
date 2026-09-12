import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

test('downloads preserve bytes, reject HTTP errors, and clean up timed-out files', async () => {
  const implementation = await import('../bin/download.js').catch(() => null);
  assert.ok(implementation, 'A bounded, measurable download implementation is required');
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'utharness-download-'));
  const server = http.createServer((request, response) => {
    if (request.url === '/fail') { response.writeHead(503); response.end(); return; }
    if (request.url === '/slow') { response.writeHead(200); response.write('partial'); return; }
    if (request.url === '/unknown') { response.writeHead(200); response.write('abc'); response.end('def'); return; }
    response.writeHead(200, { 'content-length': '6' }); response.write('abc'); response.end('def');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  try {
    const url = `http://127.0.0.1:${server.address().port}`;
    await implementation.download(url, path.join(directory, 'file'));
    assert.equal(await fs.readFile(path.join(directory, 'file'), 'utf8'), 'abcdef');
    await assert.rejects(implementation.download(`${url}/fail`, path.join(directory, 'bad')), /503/);
    await implementation.download(`${url}/unknown`, path.join(directory, 'unknown'));
    assert.equal(await fs.readFile(path.join(directory, 'unknown'), 'utf8'), 'abcdef');
    await assert.rejects(implementation.download(`${url}/slow`, path.join(directory, 'partial'), { timeoutMs: 100 }));
    await assert.rejects(fs.stat(path.join(directory, 'partial')), { code: 'ENOENT' });
    assert.equal(process.listenerCount('SIGINT'), 0);
    if (!process.stderr.isTTY) assert.equal(implementation.statusBadge('success', 'PASS'), '[PASS]');
  } finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); await fs.rm(directory, { recursive: true }); }
});
