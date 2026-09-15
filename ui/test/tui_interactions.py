"""Exercise the real Ink UI in a PTY against a deterministic native-process fixture."""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'ui/screenshots'
OUT.mkdir(exist_ok=True)

with tempfile.TemporaryDirectory(prefix='utharness-interactions-') as state:
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(ROOT)
        os.environ.update(TERM='xterm-256color', COLORTERM='truecolor', UTHARNESS_PROVIDER='groq',
                          UTHARNESS_MODEL='model-a', XDG_STATE_HOME=state, FORCE_COLOR='3', CI='false',
                          UTHARNESS_RUNTIME_BIN=str(ROOT / 'ui/test/fixtures/agent-runtime.sh'))
        os.environ.pop('NO_COLOR', None)
        os.execvp('node', ['node', str(ROOT / 'ui/dist/index.js')])
    def resize(cols, rows):
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', rows, cols, 0, 0))
    resize(120, 40)
    data = bytearray()
    def drain(seconds):
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            ready, _, _ = select.select([fd], [], [], 0.05)
            if ready:
                try: data.extend(os.read(fd, 65536))
                except OSError: return
    def expect(value, offset=0):
        until = time.monotonic() + 45
        while value.encode() not in data[offset:] and time.monotonic() < until: drain(0.1)
        assert value.encode() in data[offset:], f'Missing terminal output: {value}'
    def send(value):
        os.write(fd, value.encode()); drain(0.3); os.write(fd, b'\r')
    try:
        expect('Type your message')
        send('/not-a-command')
        expect('Unknown command: /not-a-command')
        os.write(fd, b'\x10')  # Ctrl+P, model catalog
        expect('model-b')
        os.write(fd, b'\x1b[B'); drain(0.2); os.write(fd, b'\r'); drain(0.3)
        before = len(data)
        send('route')
        expect('groq/model-b', before)
        drain(1)
        before = len(data)
        send('wait')
        expect('35% Reasoning', before)
        os.write(fd, b'\x03')
        expect('cancelled')
        drain(0.3)
        (OUT / 'audit-interactions-120x40.ansi').write_bytes(data)
        before = len(data)
        resize(30, 15)
        expect('F1 help', before)
        expect('groq/model-b', before)
        (OUT / 'audit-resize-30x15.ansi').write_bytes(data[before:])
        print('PASS: unknown command, live catalog selection, backend route, Ctrl+C cancellation, resize')
    finally:
        (OUT / 'audit-last-120x40.ansi').write_bytes(data)
        os.close(fd)
        os.kill(pid, signal.SIGTERM)
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if os.waitpid(pid, os.WNOHANG)[0]: break
            time.sleep(0.05)
        else:
            os.kill(pid, signal.SIGKILL); os.waitpid(pid, 0)
