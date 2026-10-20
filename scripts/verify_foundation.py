"""Run native acceptance checks with a fail-closed, source-bound progress journal."""
import argparse
from datetime import datetime, timezone
import hashlib
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tempfile
import time
from check_quality import execution_inputs
from fetch_upstream_issues import atomic_json

ROOT = Path(__file__).resolve().parents[1]


def acceptance_commands(root):
    evidence = root / "docs/evidence"
    commands = [
        ("fmt", ["cargo", "fmt", "--check"]),
        ("clippy", ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"]),
        ("test", ["cargo", "test", "--locked", "--workspace"]),
        ("all-targets", ["cargo", "test", "--locked", "--workspace", "--all-targets"]),
        ("ratatui-minimal-tests", ["cargo", "test", "--locked", "-p", "matea-ratatui", "--no-default-features"]),
        ("ratatui-feature-tests", ["cargo", "test", "--locked", "-p", "matea-ratatui", "--all-features"]),
        ("ratatui-feature-clippy", ["cargo", "clippy", "--locked", "-p", "matea-ratatui", "--all-targets", "--all-features", "--", "-D", "warnings"]),
        ("facade-feature-tests", ["cargo", "test", "--locked", "-p", "matea", "--all-targets", "--features", "ratatui"]),
        ("release-tests", ["cargo", "test", "--locked", "--release", "--workspace"]),
        ("style-exhaustive", ["cargo", "test", "--locked", "--release", "-p", "matea-style", "--test", "colors", "every_rgb_triple_matches_full_palette_search", "--", "--ignored"]),
        ("text-width-exhaustive", ["cargo", "test", "--locked", "--release", "-p", "matea-render", "--lib", "every_scalar_context_matches_per_grapheme_width", "--", "--ignored"]),
        ("style-patch-exhaustive", ["cargo", "test", "--locked", "--release", "-p", "matea-style", "--test", "patches", "every_boolean_base_and_ternary_patch", "--", "--ignored"]),
        ("docs", ["cargo", "doc", "--locked", "--workspace", "--all-features", "--no-deps"]),
        ("build-examples", ["cargo", "build", "--locked", "--workspace", "--all-features", "--examples"]),
        ("reference-text-fmt", ["cargo", "fmt", "--manifest-path", "tools/reference-text/Cargo.toml", "--check"]),
        ("reference-text-clippy", ["cargo", "clippy", "--locked", "--manifest-path", "tools/reference-text/Cargo.toml", "--all-targets", "--", "-D", "warnings"]),
        ("reference-text-tests", ["cargo", "test", "--locked", "--manifest-path", "tools/reference-text/Cargo.toml", "--all-targets"]),
        ("reference-text", ["cargo", "run", "--locked", "--manifest-path", "tools/reference-text/Cargo.toml", "--", "--check"]),
        ("example-smoke", [sys.executable, str(root / "scripts/example_smoke.py")]),
        ("coverage-integrity", [sys.executable, str(root / "scripts/check_coverage.py")]),
        ("coverage-guard-tests", [sys.executable, str(root / "scripts/test_coverage.py")]),
        ("reference-audit-tests", [sys.executable, str(root / "scripts/test_reference_audit.py")]),
        ("rustdoc-audit-tests", [sys.executable, str(root / "scripts/test_rustdoc_exports.py")]),
        ("source-snapshot-tests", [sys.executable, str(root / "scripts/test_source_snapshot.py")]),
        ("rustdoc-import-tests", [sys.executable, str(root / "scripts/test_rustdoc_import.py")]),
        ("issue-refresh-tests", [sys.executable, str(root / "scripts/test_issue_refresh.py")]),
        ("color-oracle-tests", [sys.executable, str(root / "scripts/test_color_trace.py")]),
        ("build-source-auditor", ["cargo", "build", "--locked", "-p", "matea-api-audit"]),
        ("source-auditor-tests", [sys.executable, str(root / "scripts/test_source_auditors.py")]),
        ("quality-gate-tests", [sys.executable, str(root / "scripts/test_quality.py")]),
        ("evidence-guard-tests", [sys.executable, str(root / "scripts/test_evidence_checks.py")]),
        ("reference-path-tests", [sys.executable, str(root / "scripts/test_reference_paths.py")]),
        ("performance-record-tests", [sys.executable, str(root / "scripts/test_performance_records.py")]),
        ("optimization-evidence-tests", [sys.executable, str(root / "scripts/test_optimization_evidence.py")]),
        ("measurement-driver-tests", [sys.executable, str(root / "scripts/test_measurement_drivers.py")]),
        ("acceptance-driver-tests", [sys.executable, str(root / "scripts/test_acceptance_driver.py")]),
        ("optimized-python-tests", [sys.executable, "-O", "-m", "unittest", "discover", "-s", str(root / "scripts"), "-p", "test_*.py"]),
        ("quality-inventory", [sys.executable, str(root / "scripts/check_quality.py")]),
        ("python-syntax", [sys.executable, "-m", "compileall", "-q", str(root / "scripts")]),
        ("build-color-probe", ["cargo", "build", "--locked", "--release", "-p", "matea-perf-probe"]),
        ("progress-api-tests", ["cargo", "run", "--locked", "--quiet", "--release", "-p", "matea-perf-probe", "--", "--progress-api-controls"]),
        ("ratatui-bridge-outputs", [sys.executable, str(root / "scripts/check_ratatui_bridge.py")]),
        ("color-trace", [sys.executable, str(root / "scripts/check_color_trace.py"),
                         "--probe", str(root / "target/release" / ("matea-perf-probe.exe" if sys.platform == "win32" else "matea-perf-probe")),
                         "--output", str(evidence / "color-policy-after.json")]),
    ]
    if sys.platform != "win32":
        commands.append(("pty-smoke", [sys.executable, str(root / "scripts/pty_smoke.py")]))
    else:
        commands.append(("windows-console", [sys.executable, str(root / "scripts/test_windows_console.py")]))
    commands.append(("render-probe", ["cargo", "run", "--locked", "--quiet", "-p", "matea", "--example", "render_probe", "--release"]))
    return commands


def run_checks(root, commands, offline=False, timeout=180):
    evidence = root / "docs/evidence"
    evidence.mkdir(parents=True, exist_ok=True)
    def normalize(value):
        return str(value).replace(str(root), "${REPOSITORY}").replace(str(Path.home()), "${HOME}")
    def source_hashes():
        return {name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in sorted(execution_inputs(root))}
    report = {"started_at_utc":datetime.now(timezone.utc).isoformat(), "platform":platform.platform(),
              "architecture":platform.machine(), "scope":"local host only; no other OS/CI execution inferred",
              "status":"running", "all_local_checks_passed":False, "checks":[],
              "path_redaction":"Repository and home prefixes are replaced with placeholders."}
    def save():
        atomic_json(evidence/"local-validation.json",report)
    # Supersede old success before probing tools, sources or test commands.
    save()
    try:
        report["source_sha256"] = source_hashes()
        names = [name for name, _ in commands]
        if not names or len(set(names)) != len(names):
            raise ValueError("acceptance requires nonempty, unique named checks")
        if any(not re.fullmatch(r"[a-z0-9-]+",name) or not command or not all(isinstance(v,str) and v for v in command)
               for name,command in commands):
            raise ValueError("invalid acceptance name/command")
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        report.update(status="failed", configuration_error=normalize(error))
        save()
        return report
    for tool in ["rustc","cargo"]:
        try:
            result=subprocess.run([tool,"--version"],capture_output=True,text=True,encoding="utf-8",errors="replace",timeout=30)
            report[tool]={"exit_code":result.returncode,"output":normalize(result.stdout+result.stderr).strip()}
        except (OSError,subprocess.SubprocessError) as error:
            report[tool]={"exit_code":127,"output":normalize(error)}
    save()
    for name, original in commands:
        command=list(original)
        if offline and command[0]=="cargo" and command[1]!="fmt":
            command.insert(command.index("--") if "--" in command else len(command),"--offline")
        environment=dict(os.environ,GOCACHE=str(Path(tempfile.gettempdir())/"matea-quality-go-cache"),
                         PYTHONPYCACHEPREFIX=str(Path(tempfile.gettempdir())/"matea-quality-python-cache"))
        if name=="docs":environment["RUSTDOCFLAGS"]="-D warnings"
        if name=="progress-api-tests":environment["MATEA_PROBE_OUTPUT_ONLY"]="1"
        start=time.monotonic()
        try:
            result=subprocess.run(command,cwd=root,env=environment,text=True,encoding="utf-8",errors="replace",
                                  capture_output=True,timeout=timeout)
            status,output=result.returncode,result.stdout+result.stderr
        except subprocess.TimeoutExpired as error:
            def decoded(value):return value.decode("utf-8",errors="replace") if isinstance(value,bytes) else value or ""
            status,output=124,decoded(error.stdout)+decoded(error.stderr)+"\nTimed out: "+str(error)
        except OSError as error:
            status,output=127,str(error)
        log_bytes=normalize(output).encode("utf-8")
        (evidence/(name+".log")).write_bytes(log_bytes)
        report["checks"].append({"name":name,"command":["python" if value==sys.executable else normalize(value) for value in command],
            "exit_code":status,"elapsed_seconds":round(time.monotonic()-start,3),"log":name+".log",
            "log_sha256":hashlib.sha256(log_bytes).hexdigest()})
        save()
        print(f"{'PASS' if status==0 else 'FAIL'} {name}",flush=True)
    try:
        current=source_hashes()
        report["source_changed_during_run"] = sorted(name for name in report["source_sha256"].keys()|current.keys()
                                                      if report["source_sha256"].get(name)!=current.get(name))
    except (OSError,ValueError,subprocess.SubprocessError) as error:
        report["source_inventory_error"]=normalize(error)
    passed=(all(check["exit_code"]==0 for check in report["checks"])
            and all(report[tool]["exit_code"]==0 for tool in ["rustc","cargo"])
            and not report.get("source_changed_during_run") and "source_inventory_error" not in report)
    report.update(status="complete" if passed else "failed",all_local_checks_passed=passed,
                  finished_at_utc=datetime.now(timezone.utc).isoformat())
    save()
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline",action="store_true")
    args=parser.parse_args()
    report=run_checks(ROOT,acceptance_commands(ROOT),args.offline)
    return 0 if report["all_local_checks_passed"] else 1


if __name__=="__main__":raise SystemExit(main())
