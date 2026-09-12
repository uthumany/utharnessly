"""A delayed local gateway must not show fake 100% completion before tokens arrive."""
import http.server
import os
from pathlib import Path
import pty
import select
import threading
import time
import tempfile

root = Path(__file__).resolve().parents[2]
class Gateway(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        self.rfile.read(int(self.headers.get('Content-Length', 0)))
        time.sleep(0.8)
        body = b'data: {"choices":[{"delta":{"content":"WAIT_TEST_OK"}}]}\n\ndata: [DONE]\n\n'
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream')
        self.send_header('Content-Length', str(len(body))); self.end_headers(); self.wfile.write(body)
    def log_message(self, *args): pass
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Gateway)
threading.Thread(target=server.serve_forever, daemon=True).start()
with tempfile.TemporaryDirectory(prefix='utharness-wait-test-') as workspace:
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(workspace)
        env = dict(os.environ, UTHARNESS_HOME=workspace, UTHARNESS_PROVIDER='custom', UTHARNESS_API_KEY='test-only', UTHARNESS_MODEL='test-model', UTHARNESS_PROVIDER_URL=f'http://127.0.0.1:{server.server_port}/v1', TERM='xterm-256color')
        binary = str(root / 'target/release/utharness')
        os.execve(binary, [binary, '--no-banner', 'chat', 'hello'], env)
    data = bytearray()
    try:
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            ready, _, _ = select.select([fd], [], [], 0.1)
            if ready:
                try: data.extend(os.read(fd, 65536))
                except OSError: break
        assert b'WAIT_TEST_OK' in data, 'Streaming did not finish'
        assert b'100%' not in data, 'Displayed completion before provider returned tokens'
        print('PASS: delayed native chat streams without fabricated completion')
    finally:
        os.close(fd)
        os.waitpid(pid, 0)
        server.shutdown()
