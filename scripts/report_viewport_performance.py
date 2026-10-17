"""Verify finite Viewport before/after outputs, raw hashes and baseline source."""
from evidence_checks import require
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/evidence/quality"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summary(record):
    values = sorted(record["samples_ns"])
    if len(values) != 31 or not all(math.isfinite(v) and v > 0 for v in values):
        raise ValueError("invalid sample distribution")
    return {"p50_ns": statistics.median(values), "p95_ns": values[29],
            "allocations": record["allocations"], "allocated_bytes": record["allocated_bytes"]}


def main():
    context = json.loads((BASE / "viewport-before-context.json").read_text(encoding="utf-8"))
    source = "crates/matea-widgets/src/lib.rs"
    old = subprocess.check_output(["git", "show", context["head"] + ":" + source], cwd=ROOT)
    require(hashlib.sha256(old).hexdigest() == context["sha256"][source])
    require(digest(BASE / "viewport-harness.patch") == context["harness_patch_sha256"])
    harness = "tools/perf-probe/src/main.rs"
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / harness
        path.parent.mkdir(parents=True)
        path.write_bytes(subprocess.check_output(["git", "show", context["head"] + ":" + harness], cwd=ROOT))
        subprocess.run(["git", "apply", str(BASE / "viewport-harness.patch")], cwd=directory, check=True)
        require(digest(path) == context["sha256"][harness])
    records = [{r["case"]: r for r in map(json.loads, (BASE / name).read_text(encoding="utf-8").splitlines())}
               for name in ["viewport-before.jsonl", "viewport-after.jsonl"]]
    require(len(records[0]) == 12 and records[0].keys() == records[1].keys())
    cases = []
    for name, before in records[0].items():
        after = records[1][name]
        require(all(after[key] == before[key] for key in ["output", "semantics", "iterations_per_sample"]), lambda: name)
        cases.append({"case": name, "before": summary(before), "after": summary(after)})
    report = {"baseline": context, "cases": cases,
              "source_sha256": {p: digest(ROOT / p) for p in [source, harness, "scripts/report_viewport_performance.py"]},
              "binary_sha256": digest(ROOT / "target/release/matea-perf-probe"),
              "raw_sha256": {n: digest(BASE / n) for n in ["viewport-before.jsonl", "viewport-after.jsonl"]},
              "intermediate": {"report": "viewport-first-report.json", "report_sha256": digest(BASE / "viewport-first-report.json"),
                               "patch_sha256": digest(BASE / "viewport-first-stage.patch")},
              "limits": ["Rust-only sequential before/after, macOS arm64, 31 samples at 3000 iterations; no affinity/frequency control or upstream parity.",
                         "Initial parsing and resident memory excluded. View generation and complete fresh render have separate workloads. Output fixtures match; boundary/CRLF correctness is tested separately.",
                         "First measurements and source patch are retained, including an adverse p95. Timing distributions are not universal maximum-performance proof."]}
    (BASE / "viewport-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print("12 Viewport fixture outputs/semantics and baseline hashes match")


if __name__ == "__main__":
    main()
