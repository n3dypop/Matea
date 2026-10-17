"""Rust-only owned-scene costs, including construction plus rendering."""
from evidence_checks import require
from performance_records import read_records, summary
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/evidence/performance/2026-10-16"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()



context = json.loads((BASE / "views-before-context.json").read_text())
require(digest(BASE / "views-before.jsonl") == context["raw_sha256"])
for name, expected in context["source_sha256"].items():
    data = subprocess.check_output(["git", "show", context["git_head"] + ":" + name], cwd=ROOT)
    require(hashlib.sha256(data).hexdigest() == expected, lambda: name)
stage = json.loads((BASE / "views-normalized-stage-context.json").read_text())
require(digest(BASE / "views-normalized-stage.jsonl") == stage["raw_sha256"])
records = [read_records((BASE / name).read_text())
           for name in ("views-before.jsonl", "views-normalized-stage.jsonl", "views-after.jsonl")]
require(len(records[0]) == 20 and all(r.keys() == records[0].keys() for r in records))
cases = []
for name, before in records[0].items():
    for r in records:
        require((r[name]["semantics"], r[name]["output"], r[name]["iterations_per_sample"]) == (before["semantics"], before["output"], before["iterations_per_sample"]), lambda: name)
    cases.append({"case": name, "semantics": before["semantics"], "output": before["output"],
                  **{label: summary(r[name]) for label, r in zip(["before", "normalized_stage", "after"], records)}})
sources = list(context["source_sha256"]) + ["crates/matea-render/src/text.rs", "crates/matea-render/tests/text.rs",
           "crates/matea/src/lib.rs", "crates/matea-widgets/src/clocks.rs", "scripts/report_view_performance.py"]
first = json.loads((BASE / "views-first-before-context.json").read_text())
require(digest(BASE / "views-first-before.jsonl") == first["raw_sha256"])
report = {"recorded_at_utc": datetime.now(timezone.utc).isoformat(), "host": platform.platform(),
          "baseline": context, "baseline_context_sha256": digest(BASE / "views-before-context.json"),
          "initial_exploration": {"context": "views-first-before-context.json", "context_sha256": digest(BASE / "views-first-before-context.json"), "raw_sha256": first["raw_sha256"]},
          "intermediate": stage, "intermediate_patch_sha256": digest(BASE / "views-normalized-stage.patch"),
          "after_git_head_before_uncommitted_changes": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
          "source_sha256": {name: digest(ROOT / name) for name in sources},
          "binary_sha256": digest(ROOT / "target/release/matea-perf-probe"),
          "rustc": subprocess.check_output(["rustc", "-vV"], text=True),
          "raw_sha256": {name: digest(BASE / name) for name in ("views-before.jsonl", "views-normalized-stage.jsonl", "views-after.jsonl")},
          "cases": cases, "upstream_parity": False,
          "limits": ["One macOS arm64 host, sequential release binaries, 31 samples, 20,000 iterations/batch; no affinity/frequency control or removed outliers.",
                     "Rust-only comparison. Constructor/clone cases exclude rendering; complete cases include scene construction or clone and fresh surface rendering. No terminal I/O.",
                     "Normalization moves from rendering to construction. Long/control constructors and some clone paths regress; full-pipeline results and intermediate samples remain visible.",
                     "Formatted counter intentionally changes format! to format_args! through the new Text conversion; the output and operation purpose match, not the caller expression.",
                     "Rendered plain output fixtures match; separate tests compare complete cells/styles, ownership, all Unicode scalar control policies and Display errors.",
                     "Short inline allocation is an implementation detail. Large strings, surfaces, panels and arbitrary user formatting can still allocate; no whole-module parity claim."]}
(BASE / "views-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
lines = ["# Owned scene text costs", "", "Rust-only measurements; no upstream or whole-module parity claim.",
         "Baseline commit: `" + context["git_head"] + "`. [Raw samples, contexts, intermediate patch and limits](evidence/performance/2026-10-16/views-report.json).", "",
         "| Workload | before p50 / p95 ns | after p50 / p95 ns | allocations before → after | requested bytes before → after |",
         "| --- | ---: | ---: | ---: | ---: |"]
for c in cases:
    b, a = c["before"], c["after"]
    lines.append(f"| {c['case']} | {b['p50_ns']:.2f} / {b['p95_ns']:.2f} | {a['p50_ns']:.2f} / {a['p95_ns']:.2f} | {b['allocations']} → {a['allocations']} | {b['allocated_bytes']} → {a['allocated_bytes']} |")
lines += ["", "Immutable Text normalizes controls once, stores small values inline and accepts format_args! without an intermediate String. Its typed drawing path skips repeated normalization of prepared leaves. Public Canvas::text still normalizes arbitrary strings.",
          "This moves work: long-ASCII construction rises from p50 28.02 to 86.01 ns and control construction from 24.38 to 101.49 ns. Long clone also regresses in this run. Complete build/render and clone/render columns are separate cases; small timing differences and p95 regressions are retained. Do not infer universal CPU wins from allocation savings.",
          "The first normalized stage's long constructor measured 109.75 ns. A safe UTF-8 byte scan reduces it in this run; every Unicode scalar is checked against the standard control policy. Intermediate source hashes, patch and samples remain available.",
          "All twenty rendered-output fixtures match. Correctness tests additionally compare full cells/styles across themes/focus/size, immediate owned formatting and failure semantics. The independent-widget example uses only facade contracts.",
          "Constructor bounds are provisionally Into<Text>; custom Into<String>-only types need explicit conversion or Into<Text>. See [ADR 0022](decisions/0022-owned-normalized-scene-text.md) and [Text API](text-api.md).", "",
          "Reproduce baseline/current release probes using `MATEA_PERF_ITERATIONS=20000 target/release/matea-perf-probe --views` with no compilation/tests running concurrently. Retain before context/raw and the intermediate data, then run `python3 scripts/report_view_performance.py`."]
(ROOT / "docs/view-performance.md").write_text("\n".join(lines) + "\n")
print("20 Rust-only owned-scene fixtures match; full module coverage remains pending")
