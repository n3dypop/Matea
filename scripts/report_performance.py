"""Publish raw samples and a narrow, output-checked Foundation comparison."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
from check_terminal_trace import replay

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--artifacts", type=Path, required=True)
args = parser.parse_args()
destination = ROOT / "docs/evidence/performance/2026-10-16"
destination.mkdir(parents=True, exist_ok=True)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def summary(record):
    samples = sorted(record["samples_ns"])
    assert len(samples) >= 31 and all(n > 0 for n in samples)
    return {"p50_ns": statistics.median(samples),
            "p95_ns": samples[math.ceil(len(samples) * .95) - 1],
            "allocations": record["allocations"], "allocated_bytes": record["allocated_bytes"]}


names = ["matea-current.jsonl", "upstream-current.jsonl", "matea-before-controlled.jsonl",
         "matea-after.jsonl", "matea-trace.json", "ultraviolet-trace.json",
         "matea-cursor-trace.json", "ultraviolet-cursor-trace.json"]
for name in names:
    shutil.copy2(args.artifacts / name, destination / name)
traces = [json.loads((destination / name).read_text()) for name in names[4:6]]
expected = [c + "a" * 79 + "\n" * 23 for c in "aba"]
assert all(replay(trace) == expected for trace in traces)
cursor_traces = [json.loads((destination / name).read_text()) for name in names[6:8]]
cursor_expected = [{"screen": "a" * 80 + "\n" * 23, "cursor": [x, 3], "visible": show}
                   for x, show in [(2, True), (4, True), (2, True), (2, False)]]
assert all(replay(trace, include_cursor=True) == cursor_expected for trace in cursor_traces)
records = []
for name in names[:2]:
    records.append({r["case"]: r for r in map(json.loads, (destination / name).read_text().splitlines())})
assert records[0].keys() == records[1].keys()
comparison = []
for case, matea in records[0].items():
    upstream = records[1][case]
    equivalent = matea["output"] == upstream["output"] and matea["semantics"] == upstream["semantics"]
    item = {"case": case, "semantics": matea["semantics"], "output_equivalent_for_this_fixture": equivalent,
            "matea": summary(matea), "upstream": summary(upstream)}
    if equivalent:
        item["upstream_p50_divided_by_matea_p50"] = item["upstream"]["p50_ns"] / item["matea"]["p50_ns"]
    if case == "terminal/one_cell_80x24":
        item["replayed_update_bytes"] = {"matea": len(traces[0]["sequences"][1].encode()),
                                         "upstream": len(traces[1]["sequences"][1].encode())}
    if case == "cursor/visible_move_80x24":
        item["replayed_move_bytes"] = {"matea": len(cursor_traces[0]["sequences"][1].encode()),
                                       "upstream": len(cursor_traces[1]["sequences"][1].encode())}
    comparison.append(item)

inputs = [ROOT / "Cargo.lock"]
for folder in ("crates", "tools/perf-probe", "tools/perf-upstream"):
    inputs.extend(p for p in (ROOT / folder).rglob("*") if p.suffix in (".rs", ".go", ".toml"))
metadata = {"schema": 1, "recorded_at_utc": datetime.now(timezone.utc).isoformat(),
            "host": platform.platform(), "architecture": platform.machine(),
            "rustc": command("rustc", "-vV"), "go": command("go", "version"),
            "git_head_before_uncommitted_changes": command("git", "rev-parse", "HEAD"),
            "source_hashes": {str(p.relative_to(ROOT)): digest(p) for p in sorted(inputs)},
            "raw_sha256": {name: digest(destination / name) for name in names},
            "upstream_pins": json.loads((ROOT / "docs/upstream-lock.json").read_text()),
            "run_order": "Rust current then Go current; sequential, release, memory sink; 31 batches, 100 warmups",
            "limits": ["One macOS arm64 host; no affinity/frequency control or real terminal I/O",
                       "Go GOMAXPROCS=1, normal GC for timings, allocation pass separately disables GC",
                       "Rust allocator forwarding branch remains in timing; counting disabled",
                       "cold_ns is first operation call, not process startup",
                       "Same immutable Surface shortcut is specific to this reuse workload",
                       "Raw spring samples use IEEE arithmetic in both; checked updates validate inputs/results in both; Rust constructors also reject invalid parameters",
                       "Nanosecond microbenchmarks are not application latency or full-module parity; retain scheduling outliers, including p95 regressions",
                       "Both current terminal encoders retain cursor/style state; stateless helper measured only in historical baseline",
                       "Old baseline files predate motion cases and anti-specialization changes; use for render/transport only"],
            "cases": comparison, "full_reference_parity": False}
baseline = destination / "matea-motion-pagination-before.jsonl"
baseline_report = destination / "motion-pagination-before-report.json"
if baseline.is_file() and baseline_report.is_file():
    old = json.loads(baseline_report.read_text())
    assert digest(baseline) == old["raw_sha256"]["matea-current.jsonl"]
    previous = {r["case"]: r for r in map(json.loads, baseline.read_text().splitlines())}
    optimized = {}
    for case in ("motion/projectile_checked_update", "pagination/range", "pagination/dots_32", "pagination/dots_256"):
        before, after = previous[case], records[0][case]
        assert before["semantics"] == after["semantics"] and before["output"] == after["output"]
        optimized[case] = {"before": summary(before), "after": summary(after)}
    metadata["motion_pagination_optimization"] = {"before_raw": baseline.name,
        "before_source_report": baseline_report.name, "before_raw_sha256": digest(baseline),
        "before_report_sha256": digest(baseline_report), "cases": optimized}
baseline = destination / "matea-textinput-before.jsonl"
baseline_report = destination / "textinput-before-report.json"
if baseline.is_file() and baseline_report.is_file():
    old = json.loads(baseline_report.read_text())
    assert digest(baseline) == old["raw_sha256"]["matea-current.jsonl"]
    previous = {r["case"]: r for r in map(json.loads, baseline.read_text().splitlines())}
    case = "textinput/suggestions_edit_pair"
    before, after = previous[case], records[0][case]
    assert before["semantics"] == after["semantics"] and before["output"] == after["output"]
    metadata["textinput_query_optimization"] = {"before_raw": baseline.name,
        "before_source_report": baseline_report.name, "before_raw_sha256": digest(baseline),
        "before_report_sha256": digest(baseline_report), "case": case,
        "before": summary(before), "after": summary(after)}
baseline = destination / "matea-cursor-before.jsonl"
baseline_report = destination / "cursor-before-report.json"
if baseline.is_file() and baseline_report.is_file():
    old = json.loads(baseline_report.read_text())
    assert digest(baseline) == old["raw_sha256"]["matea-current.jsonl"]
    previous = {r["case"]: r for r in map(json.loads, baseline.read_text().splitlines())}
    case = "cursor/visible_move_80x24"
    before, after = previous[case], records[0][case]
    assert before["semantics"] == after["semantics"] and before["output"] == after["output"]
    old_trace = json.loads((destination / "matea-cursor-before-trace.json").read_text())
    assert replay(old_trace, include_cursor=True) == cursor_expected
    metadata["cursor_move_optimization"] = {"before_raw": baseline.name,
        "before_source_report": baseline_report.name, "before_raw_sha256": digest(baseline),
        "before_report_sha256": digest(baseline_report), "case": case,
        "before": summary(before), "after": summary(after),
        "move_bytes_before": len(old_trace["sequences"][1].encode()),
        "move_bytes_after": len(cursor_traces[0]["sequences"][1].encode())}
baseline = destination / "matea-key-before.jsonl"
baseline_report = destination / "key-before-report.json"
if baseline.is_file() and baseline_report.is_file():
    old = json.loads(baseline_report.read_text())
    assert digest(baseline) == old["raw_sha256"]["matea-current.jsonl"]
    previous = {r["case"]: r for r in map(json.loads, baseline.read_text().splitlines())}
    optimized = {}
    for case in ("binding/construct", "binding/value_hit", "binding/value_miss", "binding/value_disabled"):
        before, after = previous[case], records[0][case]
        assert before["semantics"] == after["semantics"] and before["output"] == after["output"]
        optimized[case] = {"before": summary(before), "after": summary(after)}
    metadata["key_binding_optimization"] = {"before_raw": baseline.name,
        "before_source_report": baseline_report.name, "before_raw_sha256": digest(baseline),
        "before_report_sha256": digest(baseline_report), "cases": optimized}
(destination / "report.json").write_text(json.dumps(metadata, indent=2, ensure_ascii=False) + "\n")
lines = ["# Foundation performance evidence — 2026-10-16", "",
         "Status: measured workloads only. All-reference/module performance parity remains pending.", "",
         "Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).",
         "One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.", "",
         "| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |", "| --- | ---: | ---: | ---: |"]
for item in comparison:
    m, u = item["matea"], item["upstream"]
    lines.append(f"| {item['case']} | {m['p50_ns']:.2f} / {m['p95_ns']:.2f} | {u['p50_ns']:.2f} / {u['p95_ns']:.2f} | {m['allocations']} / {u['allocations']} |")
lines += ["", "All listed output fixtures match. ASCII transport is independently replayed for a→b→a.",
          "Input cases use short batches and can show scheduling outliers; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.",
          "The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.",
          "The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.",
          "The native-cursor fixture also reuses unchanged cells. Four independently replayed states check cell contents, position and visibility; shape/color and terminal I/O are excluded.",
          "Known-position same-axis cursor moves now select shorter relative commands. The retained baseline records 6 bytes per move, compared with 4 after the change and 6 for the pinned Ultraviolet public MoveTo fixture. Full repaint reanchors absolutely; wrap-pending positions never use this shortcut.",
          "Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.",
          "Projectile tests add 45 XYZ trajectories. Timed raw/checked updates and construction share the same finite fixture; checks remain enabled in both checked harnesses.",
          "The motion/pagination baseline preserves source hashes and all samples before inlining, independent finite checks, invariant-based ranges and exact dot capacity reservation; see report.json.",
          "Text-input fixtures exclude View construction, clipboard I/O, validators and virtual-cursor commands in both harnesses. They include eight event pairs; Unicode rune-vs-grapheme single-step behavior is not compared.",
          "The text-input baseline retains samples/source hashes before ASCII query-buffer reuse. Unicode filtering keeps contextual lowercase semantics; that path is not covered by the ASCII allocation claim.",
          "Key bindings add three custom-token matches plus construction and metadata access. Existing-string custom tokens do not format/allocate in either language. Constructors retain their result through black_box/global storage; Go escape analysis and Rust ownership differ. Short help storage is inline after the retained baseline.",
          "Raw spring samples use IEEE arithmetic; checked-update cases validate inputs/results in both languages. Rust constructor validation is included. Very short timings are sensitive to harness effects.", "",
          "## Reproduction", "", "```sh", "cargo build --release -p matea-perf-probe --locked",
          "python3 scripts/prepare_upstream_bench.py --references ../references --destination /tmp/matea-foundation-upstream",
          "# In the disposable Go module: go mod tidy; go build -o probe .",
          "MATEA_PERF_ITERATIONS=1000 target/release/matea-perf-probe > artifacts/performance/matea-current.jsonl",
          "MATEA_PERF_ITERATIONS=1000 /tmp/matea-foundation-upstream/probe > artifacts/performance/upstream-current.jsonl",
          "target/release/matea-perf-probe --trace > artifacts/performance/matea-trace.json",
          "/tmp/matea-foundation-upstream/probe --trace > artifacts/performance/ultraviolet-trace.json",
          "python3 scripts/check_terminal_trace.py artifacts/performance/*-trace.json", "```", "",
          "Keep reference checkouts clean and at docs/upstream-lock.json revisions. The preparation script verifies pins.",
          "No image processing, GPU or real terminal performance claim is made."]
(ROOT / "docs/performance.md").write_text("\n".join(lines) + "\n")
print(f"{len(comparison)} narrow cases; {sum(r['output_equivalent_for_this_fixture'] for r in comparison)} output matches")
