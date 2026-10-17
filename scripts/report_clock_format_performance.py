"""Compare forward/reverse clock text, preserving short and longer paired runs."""
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import platform
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/evidence/performance/2026-10-16"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summary(record):
    samples = sorted(record["samples_ns"])
    assert len(samples) >= 31 and all(math.isfinite(n) and n > 0 for n in samples)
    return {"p50_ns": statistics.median(samples), "p95_ns": samples[math.ceil(.95 * len(samples)) - 1],
            "allocations": record["allocations"], "allocated_bytes": record["allocated_bytes"]}


context = json.loads((BASE / "clocks-forward-context.json").read_text())
for name, expected in context["source_sha256"].items():
    data = subprocess.check_output(["git", "show", context["git_head"] + ":" + name], cwd=ROOT)
    assert hashlib.sha256(data).hexdigest() == expected, name
for name, expected in context["raw_sha256"].items():
    assert digest(BASE / name) == expected, name
groups = [["clocks-forward-before.jsonl", "clocks-forward-upstream.jsonl", "clocks-reverse-after.jsonl", "clocks-reverse-upstream.jsonl"],
          ["clocks-forward-long.jsonl", "clocks-forward-go-long.jsonl", "clocks-reverse-long.jsonl", "clocks-reverse-go-long.jsonl"]]
comparisons = []
for names in groups:
    records = [{r["case"]: r for r in map(json.loads, (BASE / name).read_text().splitlines())} for name in names]
    assert len(records[0]) == 16 and all(r.keys() == records[0].keys() for r in records)
    cases = []
    for case, before in records[0].items():
        for r in records:
            assert (r[case]["semantics"], r[case]["output"], r[case]["iterations_per_sample"]) == (before["semantics"], before["output"], before["iterations_per_sample"]), case
        cases.append({"case": case, "semantics": before["semantics"], "output": before["output"],
                      "iterations_per_sample": before["iterations_per_sample"],
                      **{label: summary(r[case]) for label, r in zip(["forward", "go_before", "reverse", "go_after"], records)}})
    comparisons.append({"raw_sha256": {name: digest(BASE / name) for name in names}, "cases": cases})
for short, longer in zip(comparisons[0]["cases"], comparisons[1]["cases"]):
    assert longer["iterations_per_sample"] == 20 * short["iterations_per_sample"]
    assert (short["case"], short["output"], short["semantics"]) == (longer["case"], longer["output"], longer["semantics"])
binary_paths = {"forward_rebuild": Path("/private/tmp/matea-clock-forward-probe"),
                "reverse": Path("/private/tmp/matea-clock-reverse-probe"),
                "go": Path("/private/tmp/matea-foundation-upstream/probe")}
sources = list(context["source_sha256"]) + ["scripts/report_clock_format_performance.py", "crates/matea-widgets/tests/clocks.rs", "crates/matea-widgets/tests/fixtures/durations.tsv"]
report = {"recorded_at_utc": datetime.now(timezone.utc).isoformat(), "host": platform.platform(),
          "upstream_pins": json.loads((ROOT / "docs/upstream-lock.json").read_text()),
          "upstream_lock_sha256": digest(ROOT / "docs/upstream-lock.json"),
          "rustc": subprocess.check_output(["rustc", "-vV"], text=True),
          "baseline": context, "baseline_context_sha256": digest(BASE / "clocks-forward-context.json"),
          "git_head_before_uncommitted_changes": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
          "source_sha256": {name: digest(ROOT / name) for name in sources},
          "binary_sha256": {name: digest(path) for name, path in binary_paths.items()},
          "rebuilt_forward_matches_original_binary": digest(binary_paths["forward_rebuild"]) == context["binary_sha256"]["target/release/matea-perf-probe"],
          "current_reverse_matches_measured_binary": digest(ROOT / "target/release/matea-perf-probe") == digest(binary_paths["reverse"]),
          "short_run": comparisons[0], "longer_run": comparisons[1], "module_parity": False,
          "limits": ["One macOS arm64 host, sequential release binaries; no compilation during measurement or affinity/frequency control.",
                     "Short runs showed large changes even on unchanged zero paths and the same Go binary. All short samples are retained; repeat all four runs with 20x batch size, not only favorable cases.",
                     "Longer-run Go p50 values are steadier, but unchanged Rust zero-path differences still warn against universal/causal speed claims. Keep p95 and all 31 samples.",
                     "Storage/format outputs match in all sixteen cases. Active scheduling, callback latency, arbitrary distributions and full module parity remain pending.",
                     "Fresh strings still allocate once; reused buffers allocate zero in both languages. Allocator-requested Rust bytes differ from Go allocator rounding/tiny-object accounting."]}
(BASE / "clocks-reverse-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
lines = ["# Clock formatting follow-up", "", "Same sixteen clock workloads; a follow-up of the existing 84-case set, not sixteen new capabilities.",
         "Baseline commit: `" + context["git_head"] + "`. [Raw runs, hashes, compilers and limits](evidence/performance/2026-10-16/clocks-reverse-report.json).", "",
         "| Workload | forward p50 / p95 ns | reverse p50 / p95 ns | Go after p50 / p95 ns | allocations M / Go |",
         "| --- | ---: | ---: | ---: | ---: |"]
for c in comparisons[1]["cases"]:
    b, a, g = c["forward"], c["reverse"], c["go_after"]
    lines.append(f"| {c['case']} | {b['p50_ns']:.2f} / {b['p95_ns']:.2f} | {a['p50_ns']:.2f} / {a['p95_ns']:.2f} | {g['p50_ns']:.2f} / {g['p95_ns']:.2f} | {a['allocations']} / {g['allocations']} |")
lines += ["", "Reverse preparation writes digits directly into one bounded safe stack buffer. It removes integer/fraction temporary arrays and copies, preserves exact-size String allocation, checked UTF-8 and the existing zero fast path. All 269 source duration fixtures and Rust Duration::MAX remain covered.",
          "Short 1,000-iteration runs were noisy, including unchanged zero and Go paths. All were retained, then both variants and both Go controls were rerun at 20,000 iterations (31 samples, normal 100 warmups). The table shows this longer run; raw records include both. Timings do not establish stable CPU frequency or a causal change on the unchanged zero path.",
          "Fresh maximum shared duration changes from p50 54.97 to 49.05 ns; source after is 32.50 ns. Reused-buffer maximum changes from 39.13 to 28.25 ns versus source 26.57 ns. Fresh strings remain slower than source in all six fixtures, and both reused paths allocate zero. Clock-module performance remains pending.", "",
          "Reproduce using `MATEA_PERF_ITERATIONS=20000 <probe> --clocks` on separate baseline/current release binaries and the pinned Go probe, sequentially with no compilation/tests. Retain the shorter runs and the forward context; run `python3 scripts/report_clock_format_performance.py` after collecting all named raw files. The reporter currently expects the three disposable probe paths recorded in the script."]
(ROOT / "docs/clock-format-performance.md").write_text("\n".join(lines) + "\n")
print("16 clock fixtures match in all eight retained runs; clock module parity pending")
