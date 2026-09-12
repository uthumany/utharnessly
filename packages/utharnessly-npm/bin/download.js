import { ProgressBar, Badge } from '@vr_patel/tui';
import { createWriteStream, promises as fs } from 'node:fs';
import { Transform } from 'node:stream';
import { pipeline } from 'node:stream/promises';

export function statusBadge(kind, label) {
  return process.stderr.isTTY && process.env.NO_COLOR === undefined && process.env.TERM !== 'dumb'
    ? Badge[kind](label) : `[${label}]`;
}

export async function download(url, destination, { timeoutMs = 120_000 } = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  const interrupt = () => controller.abort();
  process.once('SIGINT', interrupt);
  let bar;
  let cursorHidden = false;
  const columns = process.stdout.columns ?? 80;
  try {
    const response = await fetch(url, { redirect: 'follow', signal: controller.signal });
    if (!response.ok || !response.body) throw new Error(`download failed (${response.status}) for ${url}`);
    const total = response.headers.get('content-encoding') ? 0 : Number(response.headers.get('content-length'));
    if (process.stdout.isTTY && !process.env.CI && process.env.NO_COLOR === undefined && process.env.TERM !== 'dumb' && process.env.UTHARNESS_ASCII !== '1' && columns >= 60 && total > 0 && Number.isSafeInteger(total)) {
      bar = new ProgressBar({ total, width: Math.max(4, Math.min(40, columns - 55)), title: '', gradient: true, showCount: columns >= 80, showETA: columns >= 80 });
      cursorHidden = true;
      bar.start();
    }
    let received = 0;
    let lastDraw = 0;
    const counter = new Transform({ transform(chunk, _encoding, callback) {
      received += chunk.length;
      if (bar && process.stdout.columns !== columns) {
        process.stdout.write('\r\x1b[2K\x1b[?25h');
        bar = undefined;
        cursorHidden = false;
      }
      if (bar && Date.now() - lastDraw >= 80) { bar.update(received); lastDraw = Date.now(); }
      callback(null, chunk);
    } });
    await pipeline(response.body, counter, createWriteStream(destination), { signal: controller.signal });
    if (bar) bar.finish('Download complete');
  } catch (error) {
    await fs.rm(destination, { force: true });
    throw error;
  } finally {
    if (cursorHidden) process.stdout.write('\x1b[?25h');
    clearTimeout(timer);
    process.removeListener('SIGINT', interrupt);
  }
}
