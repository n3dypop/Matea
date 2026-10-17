"""Measure native Rust line/function execution; not reference-feature coverage."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/evidence/quality"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--existing", action="store_true", help="reuse this invocation's captured test JSON/profile files")
    args = parser.parse_args()
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

    if not args.existing:
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
    for mode in [[], ["--trace"], ["--cursor-trace"], ["--color-trace"], ["--views"], ["--subscriptions"], ["--clocks"], ["--viewport"]]:
        name = mode[0].removeprefix("--") if mode else "main"
        run([str(probe), *mode], "coverage-probe-" + name + ".log")

    objects = set()
    for log in ["coverage-test.jsonl", "coverage-build.jsonl"]:
        for line in (OUT / log).read_text(encoding="utf-8").splitlines():
            try:
                record = json.loads(line)
            except ValueError:
                continue
            if record.get("reason") == "compiler-artifact" and record.get("executable"):
                objects.add(record["executable"])
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    host = next(line.split(": ", 1)[1] for line in subprocess.check_output(["rustc", "-vV"], text=True).splitlines() if line.startswith("host: "))
    suffix = ".exe" if sys.platform == "win32" else ""
    llvm = sysroot / "lib/rustlib" / host / "bin"
    merged = profiles / "merged.profdata"
    subprocess.run([str(llvm / ("llvm-profdata" + suffix)), "merge", "-sparse", *map(str, profiles.glob("*.profraw")), "-o", str(merged)], check=True)
    command = [str(llvm / ("llvm-cov" + suffix)), "export", "--summary-only", "--instr-profile=" + str(merged)]
    for path in sorted(objects):
        command += ["-object", path]
    data = json.loads(subprocess.check_output(command, text=True))
    files = []
    for unit in data["data"]:
        for item in unit["files"]:
            try:
                path = Path(item["filename"]).relative_to(ROOT).as_posix()
            except ValueError:
                continue
            if path.startswith(("crates/", "tools/", "examples/")):
                files.append({"file": path, **item["summary"]})
    report = {"schema": 1, "rustc": subprocess.check_output(["rustc", "-vV"], text=True),
              "commands": ["cargo test --workspace --all-targets (instrumented)", "cargo build --workspace --examples --bins (instrumented)", "python scripts/example_smoke.py (instrumented)", "python scripts/test_source_auditors.py (instrumented Rust CLI)", "python scripts/pty_smoke.py (Unix)", "instrumented performance CLI: main and all seven modes; execution coverage only"],
              "limits": ["Native host only. Line/function execution is not branch, mutation, reference-capability or memory-safety proof.",
                         "Profiling changes timing. None of the instrumented probe numbers is performance evidence; zero coverage remains zero.",
                         "Embedded cfg(test) lines are included in source-file totals; separate test/example files are also listed. These are not production-only percentages."],
              "source_sha256": {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in sorted(f["file"] for f in files)},
              "files": sorted(files, key=lambda f: f["file"])}
    (OUT / "rust-execution-coverage.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    # Raw compiler/profile logs contain host paths and are not repository artifacts.
    for path in OUT.glob("coverage-*"):
        path.replace(profiles / path.name)
    print(f"Measured {len(files)} Rust source files")


if __name__ == "__main__":
    main()
