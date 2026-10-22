"""Verify finite Viewport before/after outputs, raw hashes and baseline source."""
from evidence_checks import require
from performance_records import read_records, summary, probe_path, write_report
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/evidence/quality"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def apply_harness_patch(directory, patch):
    # Reconstruct retained Git blob bytes, independent of a Windows checkout's
    # line-ending preferences. No global config or retained hash is rewritten.
    subprocess.run(["git", "-c", "core.autocrlf=false", "-c", "core.eol=lf", "apply", str(patch)],
                   cwd=directory, check=True)


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
        apply_harness_patch(directory, BASE / "viewport-harness.patch")
        require(digest(path) == context["sha256"][harness])
    records = [read_records((BASE / name).read_text(encoding="utf-8"))
               for name in ["viewport-before.jsonl", "viewport-after.jsonl"]]
    require(len(records[0]) == 12 and records[0].keys() == records[1].keys())
    cases = []
    for name, before in records[0].items():
        after = records[1][name]
        require(all(after[key] == before[key] for key in ["output", "semantics", "iterations_per_sample"]), lambda: name)
        cases.append({"case": name, "before": summary(before), "after": summary(after)})
    report = {"baseline": context, "cases": cases,
              "source_sha256": {p: digest(ROOT / p) for p in [source, harness, "scripts/report_viewport_performance.py"]},
              "binary_sha256": digest(probe_path(ROOT)),
              "raw_sha256": {n: digest(BASE / n) for n in ["viewport-before.jsonl", "viewport-after.jsonl"]},
              "intermediate": {"report": "viewport-first-report.json", "report_sha256": digest(BASE / "viewport-first-report.json"),
                               "patch_sha256": digest(BASE / "viewport-first-stage.patch")},
              "limits": ["Rust-only sequential before/after, macOS arm64, 31 samples at 3000 iterations; no affinity/frequency control or upstream parity.",
                         "Initial parsing and resident memory excluded. View generation and complete fresh render have separate workloads. Output fixtures match; boundary/CRLF correctness is tested separately.",
                         "First measurements and source patch are retained, including an adverse p95. Timing distributions are not universal maximum-performance proof."]}
    write_report(BASE / "viewport-report.json", json.dumps(report, indent=2) + "\n")
    print("12 Viewport fixture outputs/semantics and baseline hashes match")


if __name__ == "__main__":
    main()
