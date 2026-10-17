"""Measure complete source-auditor processes with pinned, output-checked inputs."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time

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
    args = parser.parse_args()
    pins = {p["name"]: p["sha"] for p in json.loads((ROOT / "docs/upstream-lock.json").read_text())}
    binary = args.binary.resolve()
    args.directory.mkdir(parents=True, exist_ok=True)
    cases = []
    with tempfile.TemporaryDirectory(prefix="matea-auditor-fixture-") as temporary:
        synthetic = Path(temporary)
        text = "".join(f"#[cfg(unix)] pub const ITEM_{i}: usize = {i};\n" for i in range(10000))
        (synthetic / "lib.rs").write_text(text, encoding="utf-8")
        for name in ["crossterm", "ratatui", "synthetic_10000"]:
            if name == "synthetic_10000":
                source = synthetic
                expected = [{"file": "lib.rs", "line": i + 1, "kind": "const", "symbol": f"ITEM_{i}",
                             "cfg": " # [cfg (unix)]"} for i in range(10000)]
                input_identity = {"generated_lib_sha256": hashlib.sha256(text.encode()).hexdigest()}
            else:
                source = args.references.resolve() / name
                if git("rev-parse", "HEAD", cwd=source) != pins[name] or git("status", "--porcelain", cwd=source):
                    raise ValueError("reference is not the clean pinned input: " + name)
                ledger = ROOT / "docs/coverage" / (name + ".json")
                expected = [{k: d[k] for k in ["file", "line", "kind", "symbol", "cfg"]}
                            for d in json.loads(ledger.read_text())["declarations"]]
                input_identity = {"upstream_sha": pins[name], "expected_ledger_sha256": digest(ledger)}
            samples = []
            output_hash = None
            output_bytes = None
            # Each timed process includes spawn, filesystem/AST work, JSON output,
            # pipe transport, flush and exit. JSON validation/hashing is outside timing.
            for sample in range(32):
                start = time.perf_counter_ns()
                result = subprocess.run([str(binary), str(source)], stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, timeout=30, check=True)
                elapsed = time.perf_counter_ns() - start
                output = [json.loads(line) for line in result.stdout.decode("utf-8").splitlines()]
                if output != expected:
                    raise ValueError("auditor output differs from independent fixture/retained ledger: " + name)
                current = hashlib.sha256(result.stdout).hexdigest()
                if output_hash is not None and current != output_hash:
                    raise ValueError("nondeterministic auditor output: " + name)
                output_hash, output_bytes = current, len(result.stdout)
                if sample:
                    samples.append(elapsed)
            cases.append({"case": name, "semantics": "complete release source-auditor process; pipe output; JSON validation outside timing; warm filesystem; no compiler export resolution",
                          "input": input_identity, "output_sha256": output_hash, "output_bytes": output_bytes,
                          "declarations": len(expected), "samples_ns": samples,
                          "allocations": None, "resident_memory": None})
    raw = args.directory / (args.stage + ".jsonl")
    raw.write_text("".join(json.dumps(c) + "\n" for c in cases), encoding="utf-8")
    sources = ["Cargo.toml", "Cargo.lock", "tools/api-audit/Cargo.toml", "tools/api-audit/src/main.rs",
               "scripts/measure_auditor_performance.py"]
    context = {"git_head": git("rev-parse", "HEAD"), "source_sha256": {p: digest(ROOT / p) for p in sources},
               "binary_sha256": digest(binary), "raw_sha256": digest(raw), "platform": platform.platform(),
               "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
               "limits": ["One native host, 31 sequential warm process samples, no affinity/frequency control.",
                          "Includes subprocess and pipe overhead; no allocation/RSS instrumentation or in-process AST-only timing.",
                          "Retained source ledgers are output regression checks, not resolved public API or upstream parity proofs."]}
    (args.directory / (args.stage + "-context.json")).write_text(json.dumps(context, indent=2) + "\n", encoding="utf-8")
    for c in cases:
        print(c["case"], "p50_ms", round(statistics.median(c["samples_ns"]) / 1e6, 3),
              "p95_ms", round(sorted(c["samples_ns"])[29] / 1e6, 3), "declarations", c["declarations"])
    if args.stage == "after":
        before = [json.loads(s) for s in (args.directory / "before.jsonl").read_text().splitlines()]
        if len(before) != len(cases):
            raise ValueError("before/after case count differs")
        for old, new in zip(before, cases):
            for field in ["case", "semantics", "input", "output_sha256", "output_bytes", "declarations"]:
                if old[field] != new[field]:
                    raise ValueError("incomparable auditor measurement: " + field)
        print("All three complete-output checks match the baseline.")


if __name__ == "__main__":
    main()
