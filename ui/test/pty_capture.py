import fcntl
import os
import pty
import select
import struct
import termios
import time
import shutil
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, "dist", "index.js")
OUT = os.path.join(ROOT, "screenshots")
os.makedirs(OUT, exist_ok=True)


def size(fd, cols, rows):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def capture(cols, rows, label, keys=b"", arguments=None):
    pid, fd = pty.fork()
    if pid == 0:
        env = os.environ.copy()
        env.pop("NO_COLOR", None)
        env.update({"TERM": "xterm-256color", "COLORTERM": "truecolor", "FORCE_COLOR": "3", "UTHARNESS_COLOR": "truecolor", "XDG_STATE_HOME": tempfile.mkdtemp(prefix="utharness-capture-")})
        node = shutil.which("node") or "/usr/bin/env"
        extra = arguments or []
        argv = (["node", BIN] if node != "/usr/bin/env" else ["env", "node", BIN]) + extra
        os.execve(node, argv, env)
    size(fd, cols, rows)
    data = bytearray()

    def drain(seconds):
        end = time.time() + seconds
        while time.time() < end:
            ready, _, _ = select.select([fd], [], [], 0.05)
            if ready:
                try:
                    data.extend(os.read(fd, 65536))
                except OSError:
                    return

    # Keep draining while the asynchronous runtime and git snapshot settle. This
    # avoids capturing a transient cleared frame on slower CI and Termux hosts.
    drain(2.2)
    if keys:
        sequence = keys if isinstance(keys, list) else [keys]
        for chunk in sequence:
            os.write(fd, chunk)
            drain(0.35)
    else:
        drain(0.3)
    # Terminate out-of-band: Ctrl+C is an application shortcut and may cancel a
    # running action instead of exiting, which would make visual tests hang.
    try:
        os.kill(pid, 15)
    except ProcessLookupError:
        pass
    deadline = time.time() + 0.6
    while time.time() < deadline:
        finished, _ = os.waitpid(pid, os.WNOHANG)
        if finished == pid:
            break
        time.sleep(0.05)
    else:
        try:
            os.kill(pid, 9)
        except ProcessLookupError:
            pass
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass
    with open(os.path.join(OUT, f"{label}-{cols}x{rows}.ansi"), "wb") as handle:
        handle.write(data)


for cols, rows in [(40, 15), (60, 20), (80, 24), (100, 30), (120, 40), (160, 50)]:
    capture(cols, rows, "focus")
capture(120, 40, "palette", b"\x0b")
capture(160, 50, "workspace", b"\x02")
capture(100, 30, "setup-welcome", arguments=["--setup"])
capture(100, 30, "setup-provider", [b"\r", b"\x1b[B", b"\r"], ["--setup"])
capture(100, 30, "setup-tools", [b"\r", b"\x1b[B", b"\r", b"\r"], ["--setup"])
print("captured")
