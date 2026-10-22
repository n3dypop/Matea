"""Measure complete source-auditor processes with pinned, output-checked inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tempfile
import time
from measurement_run import MeasurementRun, positive_interval
from performance_records import parse_json

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args, cwd=ROOT):
    return subprocess.check_output(["git", *args], cwd=cwd, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--references", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--stage", choices=["before", "after"], required=True)
    parser.add_argument("--language", choices=["rust", "go"], default="rust")
    args = parser.parse_args()
    try:
        with MeasurementRun(args.directory, args.stage+"-context.json", args.stage+".jsonl", ROOT) as journal:
            collect(args, journal)
        return 0
    except Exception as error:
        print(type(error).__name__+": "+str(error), file=sys.stderr)
        return 1


def synthetic_fixture(directory, language):
    text = ("".join(f"#[cfg(unix)] pub const ITEM_{i}: usize = {i};\n" for i in range(10000)) if language == "rust"
            else "package fixture\n" + "".join(f"const ITEM_{i} = {i}\n" for i in range(10000)))
    path = directory / ("lib.rs" if language == "rust" else "fixture.go")
    path.write_bytes(text.encode("utf-8"))
    return digest(path)


def source_snapshot(sources, revision):
    hashes = {}
    for path in sources:
        committed = subprocess.check_output(["git", "show", revision + ":" + path], cwd=ROOT, timeout=30)
        if hashlib.sha256(committed).hexdigest() != digest(ROOT / path):
            raise ValueError("commit measured source inputs before collecting evidence: " + path)
        hashes[path] = digest(ROOT / path)
    return hashes


def validate_baseline(directory, cases):
    raw = directory / "before.jsonl"
    context = parse_json((directory / "before-context.json").read_bytes())
    if context.get("status") != "complete" or context.get("raw_sha256") != digest(raw):
        raise ValueError("baseline must be a complete hash-checked measurement attempt")
    before = [parse_json(s) for s in raw.read_bytes().decode("utf-8").splitlines()]
    if len(before) != len(cases):
        raise ValueError("before/after case count differs")
    for old, new in zip(before, cases):
        for field in ["case", "semantics", "input", "output_sha256", "output_bytes", "declarations"]:
            if old[field] != new[field]:
                raise ValueError("incomparable auditor measurement: " + field)


def collect(args, journal):
    pins = {p["name"]: p["sha"] for p in json.loads((ROOT / "docs/upstream-lock.json").read_text())}
    binary = args.binary.resolve()
    sources = (["Cargo.toml", "Cargo.lock", "tools/api-audit/Cargo.toml", "tools/api-audit/src/main.rs"]
               if args.language == "rust" else ["tools/audit-go/main.go"])
    names = ["crossterm", "ratatui"] if args.language == "rust" else ["bubbletea", "bubbles", "ultraviolet"]
    sources.extend(["scripts/measure_auditor_performance.py", "scripts/measurement_run.py", "scripts/performance_records.py", "docs/upstream-lock.json"])
    sources.extend("docs/coverage/"+name+".json" for name in names)
    revision = git("rev-parse", "HEAD")
    source_hashes = source_snapshot(sources, revision)
    journal.update(git_head=revision, source_sha256=source_hashes, language=args.language)
    executable = "matea-api-audit" if args.language == "rust" else "matea-go-audit"
    expected_binary = ROOT / "target/release" / (executable + (".exe" if os.name == "nt" else ""))
    if binary != expected_binary:
        raise ValueError("binary must be the release auditor built by this invocation")
    build = (["cargo", "build", "--release", "--locked", "--offline", "-p", "matea-api-audit"] if args.language == "rust"
             else ["go", "build", "-o", str(binary), "tools/audit-go/main.go"])
    subprocess.run(build, cwd=ROOT, check=True, timeout=300,
                   env=dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target"),
                            GOCACHE=str(Path(tempfile.gettempdir()) / "matea-go-benchcache"), GOPROXY="off"))
    binary_hash = digest(binary)
    args.directory.mkdir(parents=True, exist_ok=True)
    cases = journal.records
    with tempfile.TemporaryDirectory(prefix="matea-auditor-fixture-") as temporary:
        synthetic = Path(temporary)
        generated_hash = synthetic_fixture(synthetic, args.language)
        for name in [*names, "synthetic_10000"]:
            if name == "synthetic_10000":
                source = synthetic
                expected = ([{"file": "lib.rs", "line": i + 1, "kind": "const", "symbol": f"ITEM_{i}",
                              "cfg": " # [cfg (unix)]"} for i in range(10000)] if args.language == "rust" else
                            [{"file": "fixture.go", "line": i + 2, "package": "fixture", "kind": "const", "symbol": f"ITEM_{i}"} for i in range(10000)])
                input_identity = {"generated_lib_sha256": generated_hash}
            else:
                source = args.references.resolve() / name
                if git("rev-parse", "HEAD", cwd=source) != pins[name] or git("status", "--porcelain", cwd=source):
                    raise ValueError("reference is not the clean pinned input: " + name)
                ledger = ROOT / "docs/coverage" / (name + ".json")
                fields = ["file", "line", "kind", "symbol", "cfg"] if args.language == "rust" else ["file", "line", "package", "kind", "symbol", "signature", "build"]
                expected = [{k: d[k] for k in fields if k in d}
                            for d in json.loads(ledger.read_text())["declarations"]]
                input_identity = {"upstream_sha": pins[name], "expected_ledger_sha256": digest(ledger)}
            journal.update(current_case=name)
            samples = []
            cold = None
            output_hash = None
            output_bytes = None
            # Each timed process includes spawn, filesystem/AST work, JSON output,
            # pipe transport, flush and exit. JSON validation/hashing is outside timing.
            for sample in range(32):
                start = time.perf_counter_ns()
                result = subprocess.run([str(binary), str(source)], stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, timeout=30, check=True)
                elapsed = positive_interval(time.perf_counter_ns() - start)
                output = [json.loads(line) for line in result.stdout.decode("utf-8").splitlines()]
                if output != expected:
                    raise ValueError("auditor output differs from independent fixture/retained ledger: " + name)
                current = hashlib.sha256(result.stdout).hexdigest()
                if output_hash is not None and current != output_hash:
                    raise ValueError("nondeterministic auditor output: " + name)
                output_hash, output_bytes = current, len(result.stdout)
                if sample:
                    samples.append(elapsed)
                else:
                    cold = elapsed
            journal.record({"case": name, "semantics": "complete release source-auditor process; pipe output; JSON validation outside timing; warm filesystem; no compiler export resolution",
                          "input": input_identity, "output_sha256": output_hash, "output_bytes": output_bytes,
                          "declarations": len(expected), "cold_ns": cold, "samples_ns": samples,
                          "allocations": None, "resident_memory": None})
    if args.stage == "after":
        validate_baseline(args.directory, cases)
    for name in names:
        source = args.references.resolve()/name
        if git("rev-parse", "HEAD", cwd=source) != pins[name] or git("status", "--porcelain", cwd=source):
            raise ValueError("reference changed during collection: "+name)
    if digest(binary) != binary_hash:
        raise ValueError("auditor binary changed during collection")
    if source_snapshot(sources, revision) != source_hashes:
        raise ValueError("measured source changed during collection")
    context = {"git_head": revision, "source_sha256": {p: digest(ROOT / p) for p in sources},
               "build_command": build,
               "binary_sha256": binary_hash, "platform": platform.platform(), "python": sys.version,
               "language": args.language,
               "toolchain": subprocess.check_output(["rustc", "-Vv"] if args.language == "rust" else ["go", "version"], text=True),
               "limits": ["One native host, 31 sequential warm process samples, no affinity/frequency control.",
                          "Includes subprocess and pipe overhead; no allocation/RSS instrumentation or in-process AST-only timing.",
                          "Retained source ledgers are output regression checks, not resolved public API or upstream parity proofs."]}
    journal.complete(**context)
    for c in cases:
        print(c["case"], "p50_ms", round(statistics.median(c["samples_ns"]) / 1e6, 3),
              "p95_ms", round(sorted(c["samples_ns"])[29] / 1e6, 3), "declarations", c["declarations"])
    if args.stage == "after":
        print(f"All {len(cases)} complete-output checks match the baseline.")



if __name__ == "__main__":
    raise SystemExit(main())
