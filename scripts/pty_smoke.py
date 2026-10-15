"""Unix PTY lifecycle tests; no claim about any real emulator's Unicode rendering."""
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
BINARY = TARGET / "debug/examples/lifecycle"
CSI = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")


def screen_text(data):
    """Replay the cursor/erase subset emitted by Matea, retaining skipped spaces."""
    cells = {}
    x = y = 0
    offset = 0
    for match in CSI.finditer(data):
        for c in data[offset:match.start()].decode("utf-8", errors="replace"):
            if c == "\n":
                y += 1
            elif c == "\r":
                x = 0
            elif c >= " ":
                cells[y, x] = c
                x += 1
        body = match.group()[2:-1].decode("ascii")
        command = match.group()[-1:]
        if command == b"H":
            row, col = (body or "1;1").split(";")
            y, x = int(row) - 1, int(col) - 1
        elif command == b"J" and body == "2":
            cells.clear()
        elif command == b"K" and body == "2":
            cells = {position: c for position, c in cells.items() if position[0] != y}
        offset = match.end()
    return "\n".join("".join(cells.get((row, col), " ") for col in range(200))
                     for row in range(100)).encode("utf-8")


def case(name, action=None, args=(), expected_status=0, example="lifecycle",
         ready=b"READY 72x24", expect_cancelled=True):
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 72, 0, 0))
    env = dict(os.environ, TERM="xterm-256color", NO_COLOR="1")
    child = subprocess.Popen([str(BINARY.parent / example), *args], stdin=slave, stdout=slave,
                             stderr=slave, env=env, start_new_session=True)
    output = bytearray()

    def read_until(predicate, seconds=5):
        deadline = time.monotonic() + seconds
        while not predicate() and time.monotonic() < deadline:
            if select.select([master], [], [], 0.02)[0]:
                try:
                    output.extend(os.read(master, 65536))
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
        assert predicate(), (name, "timed out", bytes(output[-1500:]))

    def plain():
        return screen_text(bytes(output))

    try:
        if "--panic-view" not in args:
            read_until(lambda: ready in plain())
            assert not termios.tcgetattr(slave)[3] & termios.ICANON, name
            if action == "post-signal":
                os.write(master, b"q")
                read_until(lambda: b"POST_RUN_READY" in output)
                os.kill(child.pid, signal.SIGTERM)
            elif action == "effect":
                read_until(lambda: b"Tea is ready" in plain())
                os.write(master, b"q")
            elif action == "resize":
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 12, 50, 0, 0))
                os.kill(child.pid, signal.SIGWINCH)
                read_until(lambda: b"READY 50x12" in plain())
                os.write(master, b"q")
            elif action == "sigterm":
                os.kill(child.pid, signal.SIGTERM)
            else:
                os.write(master, {"panic": b"p", "ctrl-c": b"\x03"}.get(action, b"q"))
        read_until(lambda: child.poll() is not None)
        while select.select([master], [], [], 0.05)[0]:
            try:
                output.extend(os.read(master, 65536))
            except OSError as error:
                if error.errno == errno.EIO:
                    break
                raise
        assert child.returncode == expected_status, (name, child.returncode, plain()[-1500:])
        assert termios.tcgetattr(slave) == original, (name, "termios not restored")
        for command in [b"\x1b[?1049l", b"\x1b[?25h", b"\x1b[?2004l", b"\x1b[?1004l"]:
            assert command in output, (name, "cleanup command missing", command)
        if expected_status == 0 and expect_cancelled:
            assert b"CANCELLED" in output, (name, "effect cancellation not observed")
        print(f"PASS {name}: input, cleanup" + (", cancellation" if expected_status == 0 and expect_cancelled else ""))
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()
        os.close(master)
        os.close(slave)


case("quit")
case("resize", "resize")
case("raw Ctrl-C", "ctrl-c")
case("SIGTERM", "sigterm")
case("update panic", "panic", expected_status=101)
case("view panic", args=("--panic-view",), expected_status=101)
case("typed effect completion", "effect", example="effects", ready=b"Loading tea",
     expect_cancelled=False)
case("post-run SIGTERM uses original handler", "post-signal", args=("--post-signal",),
     expected_status=-signal.SIGTERM, expect_cancelled=False)
