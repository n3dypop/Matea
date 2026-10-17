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
    def text(segment):
        nonlocal x,y
        for c in segment.decode("utf-8", errors="replace"):
            if c == "\n":
                y += 1
            elif c == "\r":
                x = 0
            elif c == "\b":
                x = max(0,x-1)
            elif c >= " ":
                cells[y, x] = c
                x += 1
    for match in CSI.finditer(data):
        text(data[offset:match.start()])
        body = match.group()[2:-1].decode("ascii")
        command = match.group()[-1:]
        if command == b"H":
            values = body.split(";")
            y = int(values[0] or "1") - 1
            x = int(values[1] or "1") - 1 if len(values) > 1 else 0
        elif command in (b"A", b"B", b"C", b"D"):
            distance = int(body or "1") or 1
            if command == b"A": y = max(0, y - distance)
            if command == b"B": y += distance
            if command == b"C": x += distance
            if command == b"D": x = max(0, x - distance)
        elif command == b"J" and body == "2":
            cells.clear()
        elif command == b"K" and body == "2":
            cells = {position: c for position, c in cells.items() if position[0] != y}
        offset = match.end()
    text(data[offset:])
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
        if not any(flag in args for flag in ("--panic-view", "--panic-format")):
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
            elif action == "worker-panic":
                os.write(master,b"w")
                read_until(lambda:b"RECOVERED" in output)
                visible=screen_text(output)
                assert b"RECOVERED" in visible and b"worker panic hook fixture" not in visible,(name,visible)
                os.write(master,b"q")
            elif action == "enhanced-input":
                assert b"\x1b[>15u" in output and b"\x1b[?1006h" in output
                for data,expected in [
                    (b"\x1b[24~",b"F(12) Press"),
                    (b"\x1b[113;1:3u",b"Char('q') Release"),
                    (b"\x1b[97;1:2u",b"Char('a') Repeat"),
                    (b"\x1b[97:65;2u",b"Char('A') Press Modifiers(0x0)"),
                    (b"\x1b[51:163;2u",b"Char('\xc2\xa3') Press Modifiers(0x0)"),
                    (b"\x1b[120;9u",b"SUPER"),
                    (b"\x1b[99;5:3u",b"Char('c') Release"),
                    (b"\x1b[99;5u",b"Char('c') Press"),
                    (b"\x1b[<0;4;3M",b"Down(Left) 3,2"),
                    (b"\x1b[200~tea\x1b[201~",b"Paste 3 bytes"),
                    (b"\x1b[O",b"Focus false"),
                    (b"\x1b[I",b"Focus true"),
                ]:
                    os.write(master,data)
                    read_until(lambda:expected in plain())
                    assert child.poll() is None,(name,"release or delivered Ctrl-C exited")
                os.write(master,b"\x1b[113u")
            elif action == "pagination":
                os.write(master,b"\x1b[C")
                read_until(lambda:b"Page 2 of 4" in plain() and b"Hojicha" in plain())
                os.write(master,b"lll")
                read_until(lambda:b"Page 4 of 4" in plain() and b"Shincha" in plain())
                os.write(master,b"h")
                read_until(lambda:b"Page 3 of 4" in plain() and b"Bancha" in plain())
                os.write(master,b"q")
            elif action == "form-completion":
                os.write(master,b"\x1b[200~Akira\x1b[201~\ts")
                read_until(lambda:b"Sencha" in plain())
                os.write(master,b"\t\r")
                read_until(lambda:b"Saved. Esc to close." in plain() and b"sencha" in plain())
                os.write(master,b"\x1b")
            elif action == "spinners":
                os.write(master,b"p")
                read_until(lambda:b"Animation: paused" in plain())
                os.write(master,b"r")
                read_until(lambda:b"Resets: 1" in plain())
                os.write(master,b"n")
                read_until(lambda:b"Manual: 1" in plain())
                os.write(master,b"m")
                read_until(lambda:b"Motion: reduced" in plain())
                os.write(master,b"p")
                read_until(lambda:b"Animation: running" in plain())
                os.write(master,b"m")
                read_until(lambda:b"Motion: normal" in plain())
                read_until(lambda:re.search(rb"Updates: [1-9][0-9]*",plain()) is not None)
                os.write(master,b"q")
            elif action == "clocks":
                read_until(lambda:b"Timeouts: 1" in plain())
                assert b"0s (expired)" in plain(), name
                os.write(master,b"r")
                read_until(lambda:b"Countdown:" in plain() and b"(running)" in plain() and b"(expired)" not in plain())
                os.write(master,b"c")
                read_until(lambda:b"(paused)" in plain())
                os.write(master,b"c")
                read_until(lambda:b"(paused)" not in plain())
                read_until(lambda:b"Timeouts: 2" in plain())
                os.write(master,b"q")
            elif action == "key-help":
                read_until(lambda:b"d toggle tea action" in plain())
                os.write(master,b"d")
                read_until(lambda:b"d toggle tea action" not in plain())
                os.write(master,b"?")
                read_until(lambda:b"? toggle help    q quit" in plain())
                os.write(master,b"d")
                read_until(lambda:b"d toggle tea action" in plain())
                os.write(master,b"q")
            elif action == "native-cursor":
                # Assert before exit: cleanup alone also emits Show.
                read_until(lambda: b"\x1b[4;3H\x1b[?25h" in output)
                offset = len(output)
                os.write(master, b"\x1b[200~" + "あいう".encode() + b"\x1b[201~")
                read_until(lambda: b"\x1b[4;9H\x1b[?25h" in output[offset:])
                assert b"\x1b[?25l" in output[offset:], (name, "paint did not hide cursor")
                os.write(master, b"\x1b")
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
        if action=="enhanced-input":
            assert output.count(b"\x1b[<1u")==1,(name,"keyboard stack popped twice")
            assert output.index(b"\x1b[<1u")<output.index(b"\x1b[?1049l"),(name,"keyboard popped on wrong screen")
            assert b"\x1b[?1006l" in output,(name,"mouse mode not disabled")
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
case("worker panic redraw recovery", "worker-panic")
case("view panic", args=("--panic-view",), expected_status=101)
case("owned Text formatting panic", args=("--panic-format",), expected_status=101)
case("typed effect completion", "effect", example="effects", ready=b"Loading tea",
     expect_cancelled=False)
case("post-run SIGTERM uses original handler", "post-signal", args=("--post-signal",),
     expected_status=-signal.SIGTERM, expect_cancelled=False)
case("enhanced keys, mouse and mode restoration", "enhanced-input",example="input",ready=b"Matea input",
     args=("--mouse","--kitty","--deliver-ctrl-c"),expect_cancelled=False)
case("interactive pagination", "pagination",example="pagination",ready=b"Page 1 of 4",expect_cancelled=False)
case("form completion and submission", "form-completion",example="form",ready=b"Tea tasting registration",expect_cancelled=False)
case("native cursor composition and CJK paste", "native-cursor",example="form",args=("--native",),ready=b"Tea tasting registration",expect_cancelled=False)
case("dynamic short and full key help", "key-help",example="help",ready=b"Matea key help",expect_cancelled=False)
case("monotonic clocks, pause, rearm and exactly-once timeout", "clocks",example="clocks",ready=b"Matea clocks",expect_cancelled=False)

case("spinner presets, pause, manual phase and reduced-motion schedules", "spinners",example="spinners",ready=b"Matea spinners",expect_cancelled=False)
