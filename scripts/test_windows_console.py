"""Read real Windows console cells in a disposable child; no shared-runner timing."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import uuid
from fetch_upstream_issues import atomic_json
from performance_records import parse_json

ROOT = Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def validate_trace(value):
    if (not isinstance(value, dict) or type(value.get("schema")) is not int or value["schema"] != 1
            or value.get("kind") != "native-windows-console-input"
            or type(value.get("read_width")) is not int or value["read_width"] != 60):
        raise ValueError("unexpected native console trace schema")
    frames = value.get("frames")
    expected = {f"{glyph}/{width}/{fraction}" for glyph in ["ascii", "block", "half"]
                for width in [40, 8, 60, 17] for fraction in [0, 625, 1000]}
    if not isinstance(frames, list) or len(frames) != 36:
        raise ValueError("incomplete native console frames")
    names = set()
    for frame in frames:
        if not isinstance(frame, dict) or not isinstance(frame.get("case"), str) or frame["case"] in names:
            raise ValueError("invalid/duplicate console case")
        names.add(frame["case"])
        width = frame.get("width")
        parts = frame["case"].split("/")
        if type(width) is not int or width not in [40, 8, 60, 17] or len(parts) != 3 or parts[1] != str(width):
            raise ValueError("invalid console width")
        text, stream = frame.get("expected"), frame.get("ansi")
        if (not isinstance(text, str) or len(text) != width
                or any(char not in "0123456789 %#-█░▄" for char in text)
                or not isinstance(stream, str) or not 0 < len(stream.encode("utf-8")) <= 16_384):
            raise ValueError("invalid console cell/stream input")
    if names != expected:
        raise ValueError("native console case set changed")
    return value


def validate_result(result, trace):
    if (not isinstance(result, dict) or type(result.get("schema")) is not int or result["schema"] != 1
            or result.get("status") != "complete" or result.get("restored") is not True):
        raise ValueError("native console readback or restoration failed")
    for before, after in [("mode_before", "mode_after"), ("code_page_before", "code_page_after")]:
        if type(result.get(before)) is not int or type(result.get(after)) is not int or result[before] != result[after]:
            raise ValueError("console state differs after restoration: " + before)
    if type(result.get("window_width")) is not int or result["window_width"] < trace["read_width"]:
        raise ValueError("native observation window is too narrow")
    cases = result.get("cases")
    if not isinstance(cases, list) or len(cases) != len(trace["frames"]):
        raise ValueError("incomplete native observations")
    for observed, expected in zip(cases, trace["frames"]):
        if (not isinstance(observed, dict) or observed.get("case") != expected["case"] or observed.get("width") != expected["width"]
                or observed.get("exact_cells") != 60 or observed.get("observed") != expected["expected"].ljust(60)):
            raise ValueError("native cells mismatch: " + expected["case"])


def run(probe, destination):
    destination.mkdir(parents=True, exist_ok=True)
    context = {"schema": 1, "status": "running", "platform": platform.platform(),
               "scope": "isolated Windows native console VT output and exact BMP/ASCII cell readback; not font/modern emulator/IME coverage"}
    attempt = destination / "attempts" / uuid.uuid4().hex
    attempt.mkdir(parents=True)
    context["attempt"] = attempt.relative_to(destination).as_posix()
    journal = destination / "context.json"
    def save():
        atomic_json(attempt / "context.json", context)
        atomic_json(journal, context)
    save()
    try:
        if sys.platform != "win32":
            raise ValueError("native Windows host required; no synthetic fallback qualifies this test")
        shell = shutil.which("pwsh")
        if shell is None:
            raise FileNotFoundError("PowerShell 7 is required by the validation adapter")
        oracle = ROOT / "scripts/windows_console_oracle.ps1"
        hashes = {"probe": digest(probe.read_bytes()), "oracle": digest(oracle.read_bytes())}
        environment = dict(os.environ, MATEA_PROBE_OUTPUT_ONLY="1",
                           MATEA_PERF_ITERATIONS="invalid", MATEA_PERF_MIN_SAMPLE_NS="invalid")
        output = subprocess.run([str(probe), "--windows-console-trace"], capture_output=True,
                                env=environment, check=True, timeout=30).stdout
        trace = validate_trace(parse_json(output))
        trace_path = attempt / "input.json"
        trace_path.write_bytes(output)
        observed_path = attempt / "observed.json"
        # Never read a user's console. CREATE_NEW_CONSOLE gives this test child
        # a separate buffer even when CI captures its standard output handles.
        process = subprocess.run([shell, "-NoLogo", "-NoProfile", "-NonInteractive", "-File", str(oracle),
                                  "-TracePath", str(trace_path.resolve()), "-ReportPath", str(observed_path.resolve())],
                                 creationflags=subprocess.CREATE_NEW_CONSOLE, capture_output=True, timeout=60)
        (attempt / "child-stdout.txt").write_bytes(process.stdout)
        (attempt / "child-stderr.txt").write_bytes(process.stderr)
        context["child_exit_code"] = process.returncode
        process.check_returncode()
        result = parse_json(observed_path.read_bytes())
        validate_result(result, trace)
        if {"probe": digest(probe.read_bytes()), "oracle": digest(oracle.read_bytes())} != hashes:
            raise ValueError("console probe/adapter changed during verification")
        if trace_path.read_bytes() != output:
            raise ValueError("console input changed during verification")
        context.update(status="complete", source_sha256=hashes, input_sha256=digest(output),
                       observed_sha256=digest(observed_path.read_bytes()), exact_cases=36,
                       observed_cells_per_case=60, restoration_verified=True,
                       limits=["Native console buffer output only; no keyboard/input mode, modern Windows Terminal/ConPTY, SSH, visual font or supplementary glyph claim.",
                               "No timing/latency/performance values are produced. Historical reference Windows reports remain separate assessments."])
    except Exception as error:
        context.update(status="failed", error=f"{type(error).__name__}: {error}")
    save()
    return context["status"] == "complete"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path)
    parser.add_argument("--destination", type=Path, default=ROOT / "docs/evidence/windows-console")
    args = parser.parse_args()
    suffix = ".exe" if os.name == "nt" else ""
    probe = args.probe or ROOT / "target/release" / ("matea-perf-probe" + suffix)
    success = run(probe.resolve(), args.destination.resolve())
    print("36 native console frames and restoration verified" if success else "Native console verification failed; inspect its attempt context")
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
