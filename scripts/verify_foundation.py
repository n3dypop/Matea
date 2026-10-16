"""Run Foundation acceptance checks and save actual local evidence (not CI claims)."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time

parser = argparse.ArgumentParser()
parser.add_argument("--offline", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
workspace = root
evidence = root / "docs/evidence"
evidence.mkdir(exist_ok=True)
checks = [
    ("fmt", ["cargo", "fmt", "--check"]),
    ("clippy", ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"]),
    ("test", ["cargo", "test", "--locked", "--workspace"]),
    ("all-targets", ["cargo", "test", "--locked", "--workspace", "--all-targets"]),
    ("docs", ["cargo", "doc", "--locked", "--workspace", "--no-deps"]),
    ("build-examples", ["cargo", "build", "--locked", "--workspace", "--examples"]),
    ("example-smoke", [sys.executable, str(root / "scripts/example_smoke.py")]),
    ("coverage-integrity", [sys.executable, str(root / "scripts/check_coverage.py")]),
    ("rustdoc-audit-tests", [sys.executable, str(root / "scripts/test_rustdoc_exports.py")]),
]
if sys.platform != "win32":
    checks.append(("pty-smoke", [sys.executable, str(root / "scripts/pty_smoke.py")]))
checks.append(("render-probe", ["cargo", "run", "--locked", "--quiet", "-p", "matea", "--example", "render_probe", "--release"]))
def normalize(value):
    return str(value).replace(str(root), "${REPOSITORY}").replace(str(Path.home()), "${HOME}")

report = {
    "started_at_utc": datetime.now(timezone.utc).isoformat(),
    "platform": platform.platform(),
    "architecture": platform.machine(),
    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
    "scope": "local host only; no remote CI or other OS execution inferred",
    "path_redaction": "Repository and home directory prefixes are replaced with placeholders.",
    "checks": [],
}
for name, command in checks:
    if args.offline and command[0] == "cargo" and command[1] != "fmt":
        command.insert(command.index("--") if "--" in command else len(command), "--offline")
    env = dict(os.environ)
    if name == "docs":
        env["RUSTDOCFLAGS"] = "-D warnings"
    start = time.monotonic()
    try:
        result = subprocess.run(command, cwd=workspace, env=env,
                                text=True, encoding="utf-8", errors="replace",
                                capture_output=True, timeout=180)
        status, output = result.returncode, result.stdout + result.stderr
    except subprocess.TimeoutExpired as error:
        status, output = 124, f"Timed out: {error}"
    (evidence / f"{name}.log").write_text(normalize(output), encoding="utf-8")
    report["checks"].append({"name": name, "command": ["python" if value == sys.executable else normalize(value) for value in command], "exit_code": status,
                             "elapsed_seconds": round(time.monotonic() - start, 3),
                             "log": f"{name}.log"})
    print(f"{'PASS' if status == 0 else 'FAIL'} {name}", flush=True)
report["all_local_checks_passed"] = all(c["exit_code"] == 0 for c in report["checks"])
report["finished_at_utc"] = datetime.now(timezone.utc).isoformat()
(evidence / "local-validation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
sys.exit(0 if report["all_local_checks_passed"] else 1)
