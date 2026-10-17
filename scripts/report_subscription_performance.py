"""Publish Rust-only before/after subscription evidence; no upstream parity claim."""
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



before_context = json.loads((BASE / "subscriptions-before-context.json").read_text())
require(digest(BASE / "subscriptions-before.jsonl") == before_context["raw_sha256"])
for name, expected in before_context["source_sha256"].items():
    original = subprocess.check_output(["git", "show", before_context["git_head"] + ":" + name], cwd=ROOT)
    require(hashlib.sha256(original).hexdigest() == expected, lambda: name)
records = [read_records((BASE / name).read_text())
           for name in ("subscriptions-before.jsonl", "subscriptions-after.jsonl")]
require(records[0].keys() == records[1].keys())
cases = []
for name, before in records[0].items():
    after = records[1][name]
    require((before["semantics"], before["output"], before["first_return_value"]) == (after["semantics"], after["output"], after["first_return_value"]), lambda: name)
    cases.append({"case": name, "semantics": before["semantics"], "output": before["output"],
                  "before": summary(before), "after": summary(after)})
intermediate = []
for stem, label in [("subscriptions-scoped-stage", "Scoped IDs and retained nodes, before moving ID ownership"),
                    ("subscriptions-key-move-stage", "Single map-key ownership, before empty-configuration fast path")]:
    raw, context, patch = BASE / (stem + ".jsonl"), BASE / (stem + "-report.json"), BASE / (stem + ".patch")
    data = json.loads(context.read_text())
    require(digest(raw) == data["raw_sha256"]["subscriptions-after.jsonl"])
    stage = read_records(raw.read_text())
    require(stage.keys() == records[0].keys())
    for name, record in stage.items():
        require((record["semantics"], record["output"]) == (records[0][name]["semantics"], records[0][name]["output"]))
    intermediate.append({"label": label, "raw": raw.name, "raw_sha256": digest(raw),
                         "context": context.name, "context_sha256": digest(context),
                         "patch": patch.name, "patch_sha256": digest(patch),
                         "cases": {name: summary(record) for name, record in stage.items()}})
sources = list(before_context["source_sha256"])
sources += ["crates/matea/src/lib.rs", "crates/matea-core/tests/subscription_atomicity.rs",
            "crates/matea-core/tests/subscription_identity.rs", "scripts/report_subscription_performance.py"]
report = {"recorded_at_utc": datetime.now(timezone.utc).isoformat(), "host": platform.platform(),
          "before_context": before_context, "before_context_sha256": digest(BASE / "subscriptions-before-context.json"),
          "after_git_head_before_uncommitted_changes": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
          "after_source_sha256": {name: digest(ROOT / name) for name in sources},
          "after_binary_sha256": digest(ROOT / "target/release/matea-perf-probe"),
          "after_rustc": subprocess.check_output(["rustc", "-vV"], text=True),
          "raw_sha256": {name: digest(BASE / name) for name in ("subscriptions-before.jsonl", "subscriptions-after.jsonl")},
          "negative_regression_sha256": digest(ROOT / "docs/evidence/subscription-atomicity-before.log"),
          "cases": cases, "intermediate_runs": intermediate, "upstream_parity": False,
          "limits": ["One macOS arm64 host, sequential Rust release runs, 31 samples; no affinity/frequency control or discarded outliers.",
                     "Only adapter construction/drop and ignored focus dispatch with retained subscriptions. Runtime construction occurs outside measurement; no timer notification, render, terminal I/O or Go comparison.",
                     "The probe asserts that every configured timer still fires after the retained-reconciliation measurement.",
                     "Allocation counts/bytes describe one operation, not total retained memory or startup. Inline IDs enlarge descriptors/map nodes; initial resident memory is unmeasured.",
                     "Callbacks and the model subscription Vec still allocate. Named-string IDs can still allocate/clone. This is not end-to-end zero allocation."]}
(BASE / "subscriptions-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
lines = ["# Retained subscription costs", "", "Rust-only before/after measurements; no upstream or real-time parity claim.", "",
         "Baseline commit: `" + before_context["git_head"] + "`. [Raw data, hashes and limitations](evidence/performance/2026-10-16/subscriptions-report.json).", "",
         "| Workload | p50 / p95 ns before | p50 / p95 ns after | allocations before → after | requested bytes before → after |",
         "| --- | ---: | ---: | ---: | ---: |"]
for case in cases:
    b, a = case["before"], case["after"]
    lines.append(f"| {case['case']} | {b['p50_ns']:.2f} / {b['p95_ns']:.2f} | {a['p50_ns']:.2f} / {a['p95_ns']:.2f} | {b['allocations']} → {a['allocations']} | {b['allocated_bytes']} → {a['allocated_bytes']} |")
lines += ["", "Scoped identities replace repeated formatted clock IDs; validation precedes in-place retention of timer nodes. IDs move into the map key rather than being duplicated inside Timer.",
          "Intermediate raw samples/context/source patches are retained: the zero-subscription p50 rose to 24.96 ns before an empty-configuration fast path reduced it to 19.33 ns (baseline 20.96 ns). Other short paths vary across the retained intermediate builds; do not infer a universal latency improvement.",
          "Callback/Vec allocation remains. These per-event bytes do not measure total resident memory; larger inline keys have an unmeasured setup/storage cost.",
          "The known-negative duplicate-ID regression lost an existing deadline before the fix. Current tests also cover zero periods, limits, namespaces, unchanged callbacks and changed revisions.",
          "See [ADR 0021](decisions/0021-scoped-subscription-identities-and-retained-timers.md) and the full validation log for correctness. The model/effects are not rolled back on configuration errors.", "",
          "Reproduce baseline by building the probe at the baseline commit, running `MATEA_PERF_ITERATIONS=1000 target/release/matea-perf-probe --subscriptions`, then repeat on the changed tree with no compilation/tests running concurrently.",
          "Keep before context/raw files, collect after raw as subscriptions-after.jsonl, then run `python3 scripts/report_subscription_performance.py`. All 31 samples and p95 outliers are retained."]
(ROOT / "docs/subscription-performance.md").write_text("\n".join(lines) + "\n")
print(f"{len(cases)} Rust-only subscription fixtures validated; no upstream parity claimed")
