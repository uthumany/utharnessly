"""Opt-in live Groq PTY test. Uses existing private credentials, never prints them.

Build the Rust binary and UI first. Run from the repository root.
"""
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import termios
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'ui/screenshots'
OUT.mkdir(exist_ok=True)
pid, fd = pty.fork()
if pid == 0:
    os.chdir(ROOT)
    env = dict(os.environ, TERM='xterm-256color', COLORTERM='truecolor',
               UTHARNESS_PROVIDER='groq', UTHARNESS_MODEL='openai/gpt-oss-20b',
               UTHARNESS_UI_ENTRY=str(ROOT / 'ui/dist/index.js'))
    env.pop('NO_COLOR', None)
    binary = str(ROOT / 'target/release/utharness')
    os.execve(binary, [binary, 'tui'], env)
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 120, 0, 0))
data = bytearray()

def drain(seconds):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        ready, _, _ = select.select([fd], [], [], 0.05)
        if ready:
            try:
                data.extend(os.read(fd, 65536))
            except OSError:
                return

try:
    drain(5)
    os.write(fd, b'Use list_directory on crates. Show every directory name. One step only.')
    drain(0.5)
    os.write(fd, b'\r')
    deadline = time.monotonic() + 180
    while b'event log.' not in data and time.monotonic() < deadline:
        drain(0.5)
    drain(1)
    (OUT / 'groq-agent-120x40.ansi').write_bytes(data)
    assert b'event log.' in data, 'No real agent completion within 180 seconds'
    os.write(fd, b'\x1b[5~')
    drain(0.7)
    (OUT / 'groq-tools-120x40.ansi').write_bytes(data)
    assert b'utharness-core' in data, 'Directory tool result not visible'
    print('PASS: Groq gpt-oss-20b, real directory tool results, and transcript paging')
finally:
    os.kill(pid, signal.SIGTERM)
    os.waitpid(pid, 0)
    os.close(fd)
