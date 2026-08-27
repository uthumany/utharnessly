import fcntl
import os
import pty
import select
import struct
import termios
import time
import shutil

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, "dist", "index.js")
OUT = os.path.join(ROOT, "screenshots")
os.makedirs(OUT, exist_ok=True)


def size(fd, cols, rows):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def capture(cols, rows, label, keys=b""):
    pid, fd = pty.fork()
    if pid == 0:
        env = os.environ.copy()
        env.update({"TERM": "xterm-256color", "COLORTERM": "truecolor", "UTHARNESS_COLOR": "truecolor"})
        node = shutil.which("node") or "/usr/bin/env"
        argv = ["node", BIN] if node != "/usr/bin/env" else ["env", "node", BIN]
        os.execve(node, argv, env)
    size(fd, cols, rows)
    time.sleep(0.7)
    if keys:
        os.write(fd, keys)
        time.sleep(0.3)
    data = bytearray()
    end = time.time() + 0.4
    while time.time() < end:
        ready, _, _ = select.select([fd], [], [], 0.05)
        if ready:
            try:
                data.extend(os.read(fd, 65536))
            except OSError:
                break
    try:
        os.write(fd, b"\x03")
    except OSError:
        pass
    deadline = time.time() + 0.6
    while time.time() < deadline:
        finished, _ = os.waitpid(pid, os.WNOHANG)
        if finished == pid:
            break
        time.sleep(0.05)
    else:
        try:
            os.kill(pid, 15)
        except ProcessLookupError:
            pass
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass
    with open(os.path.join(OUT, f"{label}-{cols}x{rows}.ansi"), "wb") as handle:
        handle.write(data)


for cols, rows in [(40, 18), (60, 20), (80, 24), (120, 36), (160, 40), (220, 44)]:
    capture(cols, rows, "focus")
capture(120, 36, "palette", b"\x0b")
capture(120, 36, "skills", b"\x13")
print("captured")
