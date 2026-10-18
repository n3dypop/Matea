"""Measure native Rust line/function execution; not reference-feature coverage."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
from performance_records import read_probe_outputs
from check_quality import execution_inputs
from fetch_upstream_issues import atomic_json

ROOT = Path(__file__).resolve().parents[1]


def canonical_files(data, root):
    """Keep compilation instances separate when path includes alias one file."""
    logical_root = root.absolute()
    root = root.resolve()
    grouped = {}
    for unit_index, unit in enumerate(data["data"]):
        for item in unit["files"]:
            observed = Path(item["filename"])
            try:
                path = observed.resolve().relative_to(root).as_posix()
            except ValueError:
                continue
            try:
                observed_name = observed.relative_to(logical_root).as_posix()
            except ValueError:
                observed_name = observed.relative_to(root).as_posix()
            if path.startswith(("crates/", "tools/", "examples/")):
                grouped.setdefault(path, []).append({"unit": unit_index, "compiler_filename": observed_name,
                                                    "summary": item["summary"]})
    return [{"file": path, "instances": grouped[path]} for path in sorted(grouped)]


def collect(root):
    ROOT = root.resolve()
    OUT = ROOT / "docs/evidence/quality"
    OUT.mkdir(parents=True, exist_ok=True)
    profiles = ROOT / "artifacts/quality-coverage"
    profiles.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RUSTFLAGS="-C instrument-coverage",
               LLVM_PROFILE_FILE=str(profiles / "%p-%m.profraw"),
               CARGO_TARGET_DIR=str(ROOT / "target/quality-coverage"))

    def run(command, log, **kwargs):
        with (OUT / log).open("w", encoding="utf-8") as output:
            subprocess.run(command, cwd=ROOT, env=env, stdout=output, stderr=subprocess.STDOUT,
                           text=True, check=True, timeout=300, **kwargs)

    # Clear only this tool's generated profiles; never merge earlier runs.
    for path in profiles.glob("*.profraw"):
        path.unlink()
    run(["cargo", "test", "--locked", "--offline", "--workspace", "--all-targets", "--message-format=json"], "coverage-test.jsonl")
    run(["cargo", "build", "--locked", "--offline", "--workspace", "--examples", "--bins", "--message-format=json"], "coverage-build.jsonl")
    run([sys.executable, "scripts/example_smoke.py"], "coverage-examples.log")
    run([sys.executable, "scripts/test_source_auditors.py"], "coverage-auditors.log")
    if sys.platform != "win32":
        run([sys.executable, "scripts/pty_smoke.py"], "coverage-pty.log")
    probe = ROOT / "target/quality-coverage/debug" / ("matea-perf-probe.exe" if sys.platform == "win32" else "matea-perf-probe")
    env["MATEA_PERF_ITERATIONS"] = "1"
    # Instrumented execution coverage is explicitly fixed-batch, never timing
    # evidence. Native CI exercises the real confirmed-calibration path.
    env["MATEA_PERF_MIN_SAMPLE_NS"] = "0"
    for mode in [[], ["--trace"], ["--cursor-trace"], ["--color-trace"], ["--fallback-trace"], ["--views"], ["--subscriptions"], ["--clocks"], ["--viewport"], ["--style"], ["--input"], ["--protocol"], ["--virtual-cursor"], ["--attributes"]]:
        name = mode[0].removeprefix("--") if mode else "main"
        run([str(probe), *mode], "coverage-probe-" + name + ".log")
    env["MATEA_PROBE_OUTPUT_ONLY"] = "1"
    try:
        run([str(probe)], "coverage-probe-outputs.log")
        if len(read_probe_outputs((OUT / "coverage-probe-outputs.log").read_text(encoding="utf-8"))) != 84:
            raise ValueError("incomplete instrumented correctness records")
    finally:
        env.pop("MATEA_PROBE_OUTPUT_ONLY", None)

    objects = set()
    deadline_probes = set()
    for log in ["coverage-test.jsonl", "coverage-build.jsonl"]:
        for line in (OUT / log).read_text(encoding="utf-8").splitlines():
            try:
                record = json.loads(line)
            except ValueError:
                continue
            if record.get("reason") == "compiler-artifact" and record.get("executable"):
                objects.add(record["executable"])
                if record.get("target",{}).get("name")=="foundation_paths":
                    deadline_probes.add(record["executable"])
    if len(deadline_probes)!=1:
        raise ValueError("missing unique instrumented deadline probe")
    env["MATEA_PROBE_OUTPUT_ONLY"]="1"
    try:
        run([next(iter(deadline_probes)),"--deadlines"],"coverage-deadlines.log")
        values=read_probe_outputs((OUT/"coverage-deadlines.log").read_text(encoding="utf-8"))
        if set(values)!={f"terminal/first_notification_{ms}ms" for ms in [1,5,16,50]}:
            raise ValueError("incomplete instrumented deadline records")
    finally:
        env.pop("MATEA_PROBE_OUTPUT_ONLY",None)
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True, timeout=30).strip())
    host = next(line.split(": ", 1)[1] for line in subprocess.check_output(["rustc", "-vV"], text=True, timeout=30).splitlines() if line.startswith("host: "))
    suffix = ".exe" if sys.platform == "win32" else ""
    llvm = sysroot / "lib/rustlib" / host / "bin"
    merged = profiles / "merged.profdata"
    raw_profiles = sorted(profiles.glob("*.profraw"))
    if not objects or not raw_profiles:
        raise ValueError("coverage requires this invocation's executable objects and raw profiles")
    # Avoid Windows command-length limits when many test processes write profiles.
    profile_list = profiles / "inputs.txt"
    profile_list.write_bytes(("\n".join(map(str,raw_profiles))+"\n").encode("utf-8"))
    subprocess.run([str(llvm / ("llvm-profdata" + suffix)), "merge", "-sparse", "-f", str(profile_list), "-o", str(merged)], check=True, timeout=60)
    command = [str(llvm / ("llvm-cov" + suffix)), "export", "--summary-only", "--instr-profile=" + str(merged)]
    for path in sorted(objects):
        command += ["-object", path]
    data = json.loads(subprocess.check_output(command, text=True, timeout=60))
    files = canonical_files(data, ROOT)
    if not files:
        raise ValueError("no repository source instances in LLVM export")
    report = {"schema": 2, "rustc": subprocess.check_output(["rustc", "-vV"], text=True),
              "commands": ["cargo test --workspace --all-targets (instrumented)", "cargo build --workspace --examples --bins (instrumented)", "python scripts/example_smoke.py (instrumented)", "python scripts/test_source_auditors.py (instrumented Rust CLI)", "python scripts/pty_smoke.py (Unix)", "instrumented performance CLI: main, trace, cursor/color/fallback traces, views, subscriptions, clocks, viewport, style, input, protocol, virtual cursor, text attributes; execution coverage only", "instrumented output-only main CLI: 84 records validated separately; no timing values", "instrumented output-only deadline benchmark: four real first-notification paths; no timing values"],
              "limits": ["Native host only. Line/function execution is not branch, mutation, reference-capability or memory-safety proof.",
                         "Profiling changes timing. None of the instrumented probe numbers is performance evidence; zero coverage remains zero.",
                         "Embedded cfg(test) lines are included in source-file totals; separate test/example files are also listed. These are not production-only percentages.",
                         "Canonical source paths group separate compiler instances. Their counters/percentages are retained, never averaged, summed or replaced by the most favorable instance."],
              "source_sha256": {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in sorted(f["file"] for f in files)},
              "files": sorted(files, key=lambda f: f["file"])}
    # Raw compiler/profile logs contain host paths and are not repository artifacts.
    for path in OUT.glob("coverage-*"):
        path.replace(profiles / path.name)
    print(f"Measured {len(files)} Rust source files")
    return report


def run_coverage(root, collector=collect):
    root=root.resolve()
    output=root/"docs/evidence/quality/rust-execution-coverage.json"
    report={"schema":2,"status":"running","started_at_utc":datetime.now(timezone.utc).isoformat(),
            "platform":platform.platform(),"architecture":platform.machine()}
    atomic_json(output,report)
    try:
        inputs={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in execution_inputs(root)}
        if not inputs:
            raise ValueError("coverage requires a nonempty Git source inventory")
        result=collector(root)
        current={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in execution_inputs(root)}
        if current!=inputs:
            raise ValueError("coverage inputs changed during collection")
        if result.get("schema")!=2 or not result.get("files") or not result.get("source_sha256"):
            raise ValueError("incomplete coverage result")
        report.update(result,status="complete",input_sha256=inputs)
    except (OSError,ValueError,KeyError,TypeError,StopIteration,subprocess.SubprocessError) as error:
        report.update(status="failed",error=str(error).replace(str(root),"${REPOSITORY}").replace(str(Path.home()),"${HOME}"))
    report["finished_at_utc"]=datetime.now(timezone.utc).isoformat()
    atomic_json(output,report)
    return report


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    return 0 if run_coverage(ROOT)["status"]=="complete" else 1


if __name__ == "__main__":
    raise SystemExit(main())
