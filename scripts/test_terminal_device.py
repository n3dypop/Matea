"""Isolated real native terminal-device checks; never timing evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import select
import subprocess
import sys
import threading
import time
import uuid
from evidence_checks import require
from fetch_upstream_issues import atomic_json

ROOT = Path(__file__).resolve().parents[1]


def parse_line(raw):
    require(isinstance(raw, bytes) and raw.endswith(b"\n"), "missing complete probe line")
    fields = raw.decode("ascii").rstrip("\r\n").split("\t")
    require(fields and fields[0] in {"raw", "set", "restored", "ready", "result"}, "unknown device record")
    if fields[0] in {"raw", "ready"}:
        require(len(fields) == 3 and all(v.isdecimal() for v in fields[1:]), "invalid size record")
        require(all(0 < int(v) <= 65535 for v in fields[1:]), "invalid native viewport")
    elif fields[0] == "result":
        require(len(fields) == 5 and fields[4] in {"true", "false"}, "invalid input record")
        require(len(fields[2]) % 2 == 0 and all(c in "0123456789abcdef" for c in fields[2]), "invalid input hex")
        require(fields[3] == "none" or fields[3].isdecimal(), "invalid limit record")
    else:
        require(len(fields) == 1, "extra state fields")
    return fields


def password_cases(windows=False):
    end = b"\r" if windows else b"\n"
    cases = [
        ("ascii", b"tea" + end, 4096, "ok", b"tea"),
        ("byte_backspace", "茶".encode() + b"\b" + end, 4096, "ok", bytes.fromhex("e88c")),
        ("supplementary", "🍵".encode() + end, 4096, "ok", "🍵".encode()),
        ("empty", end, 0, "ok", b""),
        ("limit", b"tea" + end, 2, "limit", b"te"),
        ("zero_limit", b"x" + end, 0, "limit", b""),
    ]
    if not windows:
        cases += [("empty_eof", b"\x04", 4096, "UnexpectedEof", b""),
                  ("partial_eof", b"tea\x04\x04", 4096, "ok", b"tea"),
                  ("no_overread", b"tea\nnext\n", 4096, "ok", b"tea")]
    return cases


def check_password(fields, kind, expected, maximum):
    require(fields[0] == "result" and fields[1] == kind and fields[2] == expected.hex(), "native input mismatch")
    require(fields[3] == (str(maximum) if kind == "limit" else "none"), "wrong limit result")
    require(fields[4] == "false", "mode restoration failed")


def windows_run(probe, context, save):
    """Only called inside our CREATE_NEW_CONSOLE child, never an inherited console."""
    import ctypes as c
    from ctypes import wintypes as w

    class Coord(c.Structure):
        _fields_ = [("x", w.SHORT), ("y", w.SHORT)]

    class Rect(c.Structure):
        _fields_ = [("left", w.SHORT), ("top", w.SHORT), ("right", w.SHORT), ("bottom", w.SHORT)]

    class BufferInfo(c.Structure):
        _fields_ = [("size", Coord), ("cursor", Coord), ("attributes", w.WORD),
                    ("window", Rect), ("maximum", Coord)]

    class Key(c.Structure):
        _fields_ = [("down", w.BOOL), ("repeat", w.WORD), ("virtual", w.WORD),
                    ("scan", w.WORD), ("unit", c.c_uint16), ("control", w.DWORD)]

    class Record(c.Structure):
        _fields_ = [("kind", w.WORD), ("padding", w.WORD), ("key", Key)]

    require(c.sizeof(Key) == 16 and c.sizeof(Record) == 20, "unexpected console input ABI")
    api = c.WinDLL("kernel32", use_last_error=True)
    signatures = {
        "CreateFileW": ([w.LPCWSTR, w.DWORD, w.DWORD, c.c_void_p, w.DWORD, w.DWORD, w.HANDLE], w.HANDLE),
        "CloseHandle": ([w.HANDLE], w.BOOL),
        "GetConsoleMode": ([w.HANDLE, c.POINTER(w.DWORD)], w.BOOL),
        "SetConsoleMode": ([w.HANDLE, w.DWORD], w.BOOL),
        "GetConsoleScreenBufferInfo": ([w.HANDLE, c.POINTER(BufferInfo)], w.BOOL),
        "FlushConsoleInputBuffer": ([w.HANDLE], w.BOOL),
        "WriteConsoleInputW": ([w.HANDLE, c.POINTER(Record), w.DWORD, c.POINTER(w.DWORD)], w.BOOL),
    }
    for name, (arguments, result) in signatures.items():
        method = getattr(api, name)
        method.argtypes, method.restype = arguments, result

    def checked(result):
        if not result:
            raise c.WinError(c.get_last_error())
        return result

    def open_console(name):
        handle = api.CreateFileW(name, 0xC0000000, 3, None, 3, 0, None)
        require(handle != c.c_void_p(-1).value, "isolated console unavailable")
        return handle

    def mode(handle):
        value = w.DWORD()
        checked(api.GetConsoleMode(handle, c.byref(value)))
        return value.value

    def inject(handle, data):
        text = data.decode("utf-8").encode("utf-16-le")
        units = [int.from_bytes(text[i:i+2], "little") for i in range(0, len(text), 2)]
        events = (Record * (2 * len(units)))()
        for index, unit in enumerate(units):
            for release in [0, 1]:
                event = events[2 * index + release]
                event.kind, event.key.down, event.key.repeat = 1, 1 - release, 1
                event.key.virtual = unit if unit in [8, 13] else 0
                event.key.unit = unit
        written = w.DWORD()
        checked(api.WriteConsoleInputW(handle, events, len(events), c.byref(written)))
        require(written.value == len(events), "partial native input injection")

    def reader(proc):
        lines = queue.Queue()
        def pump():
            while True:
                raw = proc.stdout.readline()
                lines.put(raw)
                if not raw:
                    break
        threading.Thread(target=pump, daemon=True).start()
        def record():
            try:
                return parse_line(lines.get(timeout=5))
            except queue.Empty as error:
                raise ValueError("native console response timeout") from error
        return record

    input_handle = open_console("CONIN$")
    try:
        output_handle = open_console("CONOUT$")
    except BaseException:
        checked(api.CloseHandle(input_handle))
        raise
    original_input, original_output = mode(input_handle), mode(output_handle)
    context.update(console_input_before=original_input, console_output_before=original_output)
    info = BufferInfo()
    checked(api.GetConsoleScreenBufferInfo(output_handle, c.byref(info)))
    expected_size = [str(info.window.right - info.window.left + 1), str(info.window.bottom - info.window.top + 1)]
    try:
        for name, data, maximum, kind, expected in [(n, None, 0, None, None) for n in ["state", "drop", "unwind"]] + password_cases(True):
            checked(api.FlushConsoleInputBuffer(input_handle))
            # Explicit baseline guarantees the echo-off observation is a transition.
            checked(api.SetConsoleMode(input_handle, original_input | 7))
            before = mode(input_handle)
            proc = subprocess.Popen([str(probe), name if data is None else "password", str(maximum)],
                                    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0)
            record = reader(proc)
            item = {"case": name, "before": before}
            context["cases"].append(item)
            save()
            try:
                first = record()
                item["first"] = first
                require(first[1:] == expected_size, "incorrect native console viewport")
                if data is None:
                    require(first[0] == "raw", "raw mode not reached")
                    item.update(raw=mode(input_handle), expected_raw=(before & ~7) | 0x200)
                    save()
                    require(item["raw"] == item["expected_raw"], "raw console flags differ from pinned x/term")
                    proc.stdin.write(b"!")
                    if name == "state":
                        require(record() == ["set"], "set-state barrier failed")
                        require(mode(input_handle) == before, "set-state did not restore console flags")
                        proc.stdin.write(b"!")
                    require(record() == ["restored"], "restore barrier failed")
                else:
                    require(first[0] == "ready", "input mode not reached")
                    deadline = time.monotonic() + 5
                    while mode(input_handle) & 4:
                        require(time.monotonic() < deadline, "console echo remains enabled")
                        time.sleep(.005)
                    item["password_mode"] = mode(input_handle)
                    require(item["password_mode"] == (before & ~6) | 1, "password console flags differ")
                    inject(input_handle, data)
                    result = record()
                    item["result"] = result
                    check_password(result, kind, expected, maximum)
                proc.wait(timeout=5)
                item.update(exit_code=proc.returncode, stderr=proc.stderr.read().decode("utf-8"), after=mode(input_handle))
                require(proc.returncode == 0 and not item["stderr"], "console device probe failed")
                require(item["after"] == before, "console input mode changed after cleanup")
                item["status"] = "passed"
                save()
            finally:
                if proc.poll() is None:
                    proc.kill()
                    proc.wait(timeout=5)
                for stream in [proc.stdin, proc.stdout, proc.stderr]:
                    stream.close()
    finally:
        try:
            checked(api.SetConsoleMode(input_handle, original_input))
            checked(api.SetConsoleMode(output_handle, original_output))
            context.update(console_input_after=mode(input_handle), console_output_after=mode(output_handle))
            require(context["console_input_after"] == original_input and context["console_output_after"] == original_output,
                    "isolated console restoration failed")
        finally:
            checked(api.CloseHandle(input_handle))
            checked(api.CloseHandle(output_handle))


def windows_dispatch(probe, attempt, context, save):
    token = uuid.uuid4().hex
    child_destination = attempt / "new-console"
    command = [sys.executable, str(Path(__file__).resolve()), "--probe", str(probe),
               "--destination", str(child_destination), "--console-child", token]
    child = subprocess.run(command, creationflags=subprocess.CREATE_NEW_CONSOLE, capture_output=True,
                           env=dict(os.environ, MATEA_DEVICE_CONSOLE_CHILD=token), timeout=60)
    (attempt / "child-stdout.txt").write_bytes(child.stdout)
    (attempt / "child-stderr.txt").write_bytes(child.stderr)
    context["child_exit_code"] = child.returncode
    save()
    child.check_returncode()
    result = json.loads((child_destination / "context.json").read_bytes())
    require(result.get("status") == "complete" and result.get("binary_sha256") == context["binary_sha256"],
            "isolated console did not produce a new successful observation")
    require([case.get("case") for case in result["cases"]] == ["state", "drop", "unwind", *[case[0] for case in password_cases(True)]],
            "incomplete isolated console cases")
    require(all(case.get("status") == "passed" for case in result["cases"]), "incomplete console result")
    context.update(cases=result["cases"], native_console=result)


def unix_run(probe, context, save):
    import fcntl
    import pty
    import struct
    import termios

    def portable_state(value):
        return [v if isinstance(v, int) else [x.hex() if isinstance(x, bytes) else x for x in v] for v in value]

    def configured(value):
        result = list(value)
        if sys.platform == "darwin":
            result[3] &= ~getattr(termios, "PENDIN", 0)
        return result

    def record(proc, timeout=5):
        require(bool(select.select([proc.stdout], [], [], timeout)[0]), "native device response timeout")
        return parse_line(proc.stdout.readline())

    for name, data, maximum, kind, expected in [(n, None, 0, None, None) for n in ["state", "drop", "unwind"]] + password_cases():
        master, slave = pty.openpty()
        original = termios.tcgetattr(slave)
        settings = termios.tcgetattr(slave)
        settings[6][termios.VERASE] = b"\x7f"
        termios.tcsetattr(slave, termios.TCSANOW, settings)
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 42, 132, 0, 0))
        before = termios.tcgetattr(slave)
        proc = subprocess.Popen([str(probe), name if data is None else "password", str(maximum)],
                                stdin=slave, stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0)
        item = {"case": name, "before": portable_state(before)}
        context["cases"].append(item)
        save()
        try:
            first = record(proc)
            item["first"] = first
            require(first[1:] == ["132", "42"], "incorrect native size")
            if data is None:
                require(first[0] == "raw", "raw mode not reached")
                raw = termios.tcgetattr(slave)
                expected_raw = list(before)
                expected_raw[6] = list(before[6])
                for flag in ["IGNBRK", "BRKINT", "PARMRK", "ISTRIP", "INLCR", "IGNCR", "ICRNL", "IXON"]:
                    expected_raw[0] &= ~getattr(termios, flag)
                expected_raw[1] &= ~termios.OPOST
                for flag in ["ECHO", "ECHONL", "ICANON", "ISIG", "IEXTEN"]:
                    expected_raw[3] &= ~getattr(termios, flag)
                expected_raw[2] = (expected_raw[2] & ~(termios.CSIZE | termios.PARENB)) | termios.CS8
                expected_raw[6][termios.VMIN] = 1
                expected_raw[6][termios.VTIME] = 0
                item.update(raw=portable_state(raw), expected_raw=portable_state(expected_raw))
                save()
                require(configured(raw) == configured(expected_raw), "raw flags differ from pinned x/term")
                os.write(master, b"!")
                if name == "state":
                    require(record(proc) == ["set"], "set-state barrier failed")
                    require(configured(termios.tcgetattr(slave)) == configured(before), "set-state did not restore configured flags")
                    # Canonical mode needs a delimiter to release this barrier.
                    os.write(master, b"!\n")
                require(record(proc) == ["restored"], "restore barrier failed")
            else:
                require(first[0] == "ready", "input mode not reached")
                deadline = time.monotonic() + 5
                while termios.tcgetattr(slave)[3] & termios.ECHO:
                    require(time.monotonic() < deadline, "echo remains enabled")
                    time.sleep(.005)
                os.write(master, data)
                result = record(proc)
                item["result"] = result
                check_password(result, kind, expected, maximum)
            proc.wait(timeout=5)
            item.update(exit_code=proc.returncode, stderr=proc.stderr.read().decode("utf-8"),
                        after=portable_state(termios.tcgetattr(slave)))
            require(proc.returncode == 0 and not item["stderr"], "device probe process failed")
            require(configured(termios.tcgetattr(slave)) == configured(before), "configured terminal state changed")
            if name == "no_overread":
                os.set_blocking(slave, False)
                remainder = os.read(slave, 64)
                item["remainder_hex"] = remainder.hex()
                require(remainder == b"next\n", "following line consumed")
            item["status"] = "passed"
            save()
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait(timeout=5)
            proc.stdout.close()
            proc.stderr.close()
            termios.tcsetattr(slave, termios.TCSANOW, original)
            os.close(master)
            os.close(slave)


def run(probe, destination, console_child=False):
    destination.mkdir(parents=True, exist_ok=True)
    attempt = destination / "attempts" / uuid.uuid4().hex
    attempt.mkdir(parents=True)
    context = {"schema": 1, "status": "running", "platform": platform.platform(),
               "attempt": attempt.relative_to(destination).as_posix(), "cases": [],
               "binary_sha256": hashlib.sha256(probe.read_bytes()).hexdigest(),
               "limits": ["Only a disposable native PTY/new console; no user device or real password data.",
                          "Native behavior and cleanup only; no timing or physical-font/IME guarantee."]}
    def save():
        atomic_json(attempt / "context.json", context)
        atomic_json(destination / "context.json", context)
    save()
    try:
        if sys.platform == "win32":
            if console_child:
                windows_run(probe, context, save)
            else:
                windows_dispatch(probe, attempt, context, save)
        else:
            require(not console_child, "console child requires a Windows host")
            unix_run(probe, context, save)
        require(hashlib.sha256(probe.read_bytes()).hexdigest() == context["binary_sha256"], "probe changed during test")
        context["status"] = "complete"
    except Exception as error:
        context.update(status="failed", error=f"{type(error).__name__}: {error}")
    save()
    return context["status"] == "complete"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path)
    parser.add_argument("--destination", type=Path, default=ROOT / "docs/evidence/terminal-device")
    parser.add_argument("--console-child", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.console_child:
        require(args.console_child == os.environ.get("MATEA_DEVICE_CONSOLE_CHILD"), "unowned console child")
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    suffix = ".exe" if sys.platform == "win32" else ""
    probe = args.probe or target / "debug/examples" / ("device_probe" + suffix)
    success = run(probe.resolve(), args.destination.resolve(), bool(args.console_child))
    print("Native device checks complete" if success else "Native device checks failed; see the retained attempt")
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
