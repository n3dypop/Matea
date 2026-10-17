"""Publish raw samples and a narrow, output-checked Foundation comparison."""
from evidence_checks import require
from performance_records import read_records, summary
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import os
import tempfile
import subprocess
from check_terminal_trace import replay
from check_color_trace import verify as verify_colors

ROOT = Path(__file__).resolve().parents[1]
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def generate(artifacts, matea_binary=None, upstream_binary=None, destination=None):
    destination = destination or ROOT / "docs/evidence/performance/2026-10-16"
    names = ["matea-current.jsonl", "upstream-current.jsonl", "matea-before-controlled.jsonl",
             "matea-after.jsonl", "matea-trace.json", "ultraviolet-trace.json",
             "matea-cursor-trace.json", "ultraviolet-cursor-trace.json",
             "matea-color-trace.json", "ultraviolet-color-trace.json"]
    incoming = {name: (artifacts / name).read_bytes() for name in names}
    for name in names[:4]:
        read_records(incoming[name].decode("utf-8"))
    traces = [json.loads(incoming[name].decode("utf-8")) for name in names[4:6]]
    expected = [c + "a" * 79 + "\n" * 23 for c in "aba"]
    require(all(replay(trace) == expected for trace in traces))
    cursor_traces = [json.loads(incoming[name].decode("utf-8")) for name in names[6:8]]
    cursor_expected = [{"screen": "a" * 80 + "\n" * 23, "cursor": [x, 3], "visible": show}
                       for x, show in [(2, True), (4, True), (2, True), (2, False)]]
    require(all(replay(trace, include_cursor=True) == cursor_expected for trace in cursor_traces))
    color_traces = [json.loads(incoming[name].decode("utf-8")) for name in names[8:10]]
    for trace in color_traces:
        verify_colors(trace)
    verify_colors(color_traces[0], nondefault_margins=True)
    margin_before_path = destination / "matea-color-before-margin-trace.json"
    margin_proof = json.loads((ROOT / "docs/evidence/row-margin-before.json").read_text())
    require(digest(margin_before_path) == margin_proof["before_trace_sha256"])
    try:
        verify_colors(json.loads(margin_before_path.read_text()), nondefault_margins=True)
    except AssertionError:
        pass
    else:
        raise AssertionError("old margin regression no longer reproduces")
    records = []
    for name in names[:2]:
        records.append(read_records(incoming[name].decode("utf-8")))
    require(records[0].keys() == records[1].keys())
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
        if case.startswith("terminal/colored_full_"):
            item["measured_full_frame_bytes"] = {"matea": matea["first_return_value"], "upstream": upstream["first_return_value"]}
        comparison.append(item)
    require(all(item["output_equivalent_for_this_fixture"] for item in comparison), lambda: "fixture semantics/output mismatch; do not publish a parity claim")

    inputs = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for folder in ("crates", "tools/perf-probe", "tools/perf-upstream"):
        inputs.extend(p for p in (ROOT / folder).rglob("*") if p.suffix in (".rs", ".go", ".toml"))
    metadata = {"schema": 1, "recorded_at_utc": datetime.now(timezone.utc).isoformat(),
                "host": platform.platform(), "architecture": platform.machine(),
                "rustc": command("rustc", "-vV"), "go": command("go", "version"),
                "git_head_before_uncommitted_changes": command("git", "rev-parse", "HEAD"),
                "source_hashes": {str(p.relative_to(ROOT)): digest(p) for p in sorted(inputs)},
                "raw_sha256": {name: hashlib.sha256(incoming[name]).hexdigest() for name in names},
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
    metadata["binary_sha256"] = {name: digest(path) for name, path in
        [("matea", matea_binary), ("upstream", upstream_binary)] if path is not None}
    metadata["source_hashes_scope"] = "Source snapshots at report generation include test/example context; release probes do not compile those targets. Optional binary hashes identify the measured executables."
    metadata["vertical_margin_recovery"] = {
        "before_proof": "docs/evidence/row-margin-before.json",
        "before_trace": margin_before_path.name, "before_trace_sha256": digest(margin_before_path),
        "after_trace": "matea-color-trace.json", "after_nondefault_margins": [3, 7],
        "after_all_profiles_and_recovery_passed": True,
        "preconditions": "origin mode off; horizontal-margin mode off; finite replay, not real-emulator compatibility",
        "timing_comparison": "Both timed harnesses use default margins; extra Matea margin replay is a separate correctness regression, not a cross-module parity claim",
    }
    baseline = destination / "matea-motion-pagination-before.jsonl"
    baseline_report = destination / "motion-pagination-before-report.json"
    if baseline.is_file() and baseline_report.is_file():
        old = json.loads(baseline_report.read_text())
        require(digest(baseline) == old["raw_sha256"]["matea-current.jsonl"])
        previous = read_records(baseline.read_text())
        optimized = {}
        for case in ("motion/projectile_checked_update", "pagination/range", "pagination/dots_32", "pagination/dots_256"):
            before, after = previous[case], records[0][case]
            require(before["semantics"] == after["semantics"] and before["output"] == after["output"])
            optimized[case] = {"before": summary(before), "after": summary(after)}
        metadata["motion_pagination_optimization"] = {"before_raw": baseline.name,
            "before_source_report": baseline_report.name, "before_raw_sha256": digest(baseline),
            "before_report_sha256": digest(baseline_report), "cases": optimized}
    baseline = destination / "matea-textinput-before.jsonl"
    baseline_report = destination / "textinput-before-report.json"
    if baseline.is_file() and baseline_report.is_file():
        old = json.loads(baseline_report.read_text())
        require(digest(baseline) == old["raw_sha256"]["matea-current.jsonl"])
        previous = read_records(baseline.read_text())
        case = "textinput/suggestions_edit_pair"
        before, after = previous[case], records[0][case]
        require(before["semantics"] == after["semantics"] and before["output"] == after["output"])
        metadata["textinput_query_optimization"] = {"before_raw": baseline.name,
            "before_source_report": baseline_report.name, "before_raw_sha256": digest(baseline),
            "before_report_sha256": digest(baseline_report), "case": case,
            "before": summary(before), "after": summary(after)}
    baseline = destination / "matea-cursor-before.jsonl"
    baseline_report = destination / "cursor-before-report.json"
    if baseline.is_file() and baseline_report.is_file():
        old = json.loads(baseline_report.read_text())
        require(digest(baseline) == old["raw_sha256"]["matea-current.jsonl"])
        previous = read_records(baseline.read_text())
        case = "cursor/visible_move_80x24"
        before, after = previous[case], records[0][case]
        require(before["semantics"] == after["semantics"] and before["output"] == after["output"])
        old_trace = json.loads((destination / "matea-cursor-before-trace.json").read_text())
        require(replay(old_trace, include_cursor=True) == cursor_expected)
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
        require(digest(baseline) == old["raw_sha256"]["matea-current.jsonl"])
        previous = read_records(baseline.read_text())
        optimized = {}
        for case in ("binding/construct", "binding/value_hit", "binding/value_miss", "binding/value_disabled"):
            before, after = previous[case], records[0][case]
            require(before["semantics"] == after["semantics"] and before["output"] == after["output"])
            optimized[case] = {"before": summary(before), "after": summary(after)}
        metadata["key_binding_optimization"] = {"before_raw": baseline.name,
            "before_source_report": baseline_report.name, "before_raw_sha256": digest(baseline),
            "before_report_sha256": digest(baseline_report), "cases": optimized}
    color_stages = {}
    for stage, raw_name, report_name in [
        ("before_cache", "matea-color-before.jsonl", "color-before-report.json"),
        ("cache_before_control_fastpaths", "matea-color-cache-intermediate.jsonl", "color-cache-intermediate-report.json"),
        ("control_fastpaths_before_row_encoding", "matea-color-fastpaths.jsonl", "color-fastpaths-report.json"),
    ]:
        baseline, baseline_report = destination / raw_name, destination / report_name
        old = json.loads(baseline_report.read_text())
        require(digest(baseline) == old["raw_sha256"]["matea-current.jsonl"])
        previous = read_records(baseline.read_text())
        cases = {}
        for case, after in records[0].items():
            if not case.startswith("terminal/colored_full_"):
                continue
            before = previous[case]
            require(before["semantics"] == after["semantics"] and before["output"] == after["output"])
            cases[case] = {"before": summary(before), "after": summary(after),
                           "frame_bytes_before": before["first_return_value"],
                           "frame_bytes_after": after["first_return_value"]}
        color_stages[stage] = {"raw": raw_name, "source_report": report_name,
                               "raw_sha256": digest(baseline), "report_sha256": digest(baseline_report),
                               "cases": cases}
    metadata["color_frame_optimization"] = {
        "scope": "correct explicit-color serializer baseline; warm logical palette caches; all four profiles are controls",
        "stages": color_stages,
        "intermediate_regression": "Cache-only version regressed truecolor/none p50 by roughly 8/9 percent; retained samples motivated direct level fast paths.",
        "full_module_parity": False,
    }
    baseline, baseline_report = destination / "matea-help-before.jsonl", destination / "help-before-report.json"
    if baseline.is_file() and baseline_report.is_file():
        old = json.loads(baseline_report.read_text())
        require(digest(baseline) == old["raw_sha256"]["matea-current.jsonl"])
        previous = read_records(baseline.read_text())
        optimized = {}
        for case, after in records[0].items():
            if case.startswith("help/"):
                before = previous[case]
                require(before["semantics"] == after["semantics"] and before["output"] == after["output"])
                optimized[case] = {"before": summary(before), "after": summary(after)}
        metadata["help_text_optimization"] = {"before_raw": baseline.name, "before_source_report": baseline_report.name,
            "before_raw_sha256": digest(baseline), "before_report_sha256": digest(baseline_report), "cases": optimized,
            "scope": "Fresh unstyled strings. Capacity hints reduce growth allocations; additional zero-width cluster safety is included, so retain any timing regressions."}
        intermediate = destination / "matea-help-capacity-intermediate.jsonl"
        intermediate_report = destination / "help-capacity-intermediate-report.json"
        stage = json.loads(intermediate_report.read_text())
        require(digest(intermediate) == stage["raw_sha256"]["matea-current.jsonl"])
        records_between = read_records(intermediate.read_text())
        for case in optimized:
            require(records_between[case]["output"] == records[0][case]["output"])
            optimized[case]["capacity_intermediate"] = summary(records_between[case])
        metadata["help_text_optimization"]["intermediate"] = {
            "raw": intermediate.name, "source_report": intermediate_report.name,
            "raw_sha256": digest(intermediate), "report_sha256": digest(intermediate_report),
            "finding": "Unicode p50 regressed from 299.875 to 518.417 ns after a separate grapheme safety scan; final layout caches cell widths during the same scan and uses ASCII fast paths."}
    baseline = destination / "matea-clock-format-before.jsonl"
    baseline_report = destination / "clock-format-before-report.json"
    if baseline.is_file() and baseline_report.is_file():
        old = json.loads(baseline_report.read_text())
        require(digest(baseline) == old["raw_sha256"]["matea-current.jsonl"])
        previous = read_records(baseline.read_text())
        optimized = {}
        for case, after in records[0].items():
            if case.startswith("clocks/format_"):
                before = previous[case]
                require(before["semantics"] == after["semantics"] and before["output"] == after["output"])
                optimized[case] = {"before": summary(before), "after": summary(after)}
        metadata["clock_format_optimization"] = {"before_raw": baseline.name, "before_source_report": baseline_report.name,
            "before_raw_sha256": digest(baseline), "before_report_sha256": digest(baseline_report), "cases": optimized,
            "scope": "Fresh owned strings: fragmented Display/ToString before; bounded safe stack preparation and exact-size String after. Same visible output. Relative source timing regressions remain visible."}
        stage_raw, stage_report = destination / "matea-clock-format-stack.jsonl", destination / "clock-format-stack-report.json"
        stage = json.loads(stage_report.read_text())
        require(digest(stage_raw) == stage["raw_sha256"]["matea-current.jsonl"])
        stage_records = read_records(stage_raw.read_text())
        for case in optimized:
            require(stage_records[case]["output"] == records[0][case]["output"])
            optimized[case]["stack_before_zero_fast_path"] = summary(stage_records[case])
        metadata["clock_format_optimization"]["intermediate"] = {"raw": stage_raw.name, "source_report": stage_report.name,
            "raw_sha256": digest(stage_raw), "report_sha256": digest(stage_report),
            "finding": "Initial stack formatting regressed zero duration p50 from 26.959 to 39.292 ns; retain it and specialize the zero case. Fresh-string source comparisons can still be slower."}
    metadata["clock_performance_pending"] = {
        "active_update": "Go Update arms a time.Timer and returns commands; Rust Update changes state and exposes a reusable schedule descriptor. These cannot be fairly compared as isolated pure state updates. Matched end-to-end scheduling/notification latency, cancellation and subscription construction remain pending.",
        "measured": "Six fresh and six reused-output compact formatters plus four paused/foreign guards; source setup timers drained outside measurements. No active-tick or module parity claim.",
        "allocation_bytes": "Rust counts allocator-requested bytes; Go MemStats includes allocator rounding/tiny-object effects. Counts/bytes are not physical-memory equivalence."}
    checkpoint = destination / "clocks-checkpoint-report.json"
    checkpoint_data = json.loads(checkpoint.read_text())
    checkpoint_raw = [destination / "matea-clocks-checkpoint.jsonl", destination / "upstream-clocks-checkpoint.jsonl"]
    for raw, key in zip(checkpoint_raw, ("matea-current.jsonl", "upstream-current.jsonl")):
        require(digest(raw) == checkpoint_data["raw_sha256"][key])
    metadata["clock_checkpoint"] = {"report": checkpoint.name, "report_sha256": digest(checkpoint),
        "raw_sha256": {raw.name: digest(raw) for raw in checkpoint_raw},
        "scope": "Retain the 70-case clock implementation checkpoint before adding spinner cases; current raw records are a new full sequential run."}
    metadata["spinner_performance_pending"] = {
        "measured": "Twelve plain immutable preset frames (storage sharing allowed) and two rejected-ID/generation guards; no actual timer creation in either measured operation.",
        "pending": "Active scheduling, notification latency, custom frame construction/reconfiguration, labels, rich styled views and full module equivalence remain pending.",
        "allocation_contract": "Rust Cow borrows unlabeled frames. Go unstyled View may share its immutable string. Neither operation requires a fresh owned copy. Source Model.View still goes through default Lip Gloss Style.Render; guard cases include plain frame retrieval/rendering and do not isolate guard speed."}
    view_checkpoint = destination / "views-reference-checkpoint-report.json"
    view_checkpoint_data = json.loads(view_checkpoint.read_text())
    view_checkpoint_raw = [destination / "matea-views-reference-before.jsonl", destination / "upstream-views-reference-before.jsonl"]
    for raw, key in zip(view_checkpoint_raw, ("matea-current.jsonl", "upstream-current.jsonl")):
        require(digest(raw) == view_checkpoint_data["raw_sha256"][key])
    metadata["view_text_followup"] = {"reference_checkpoint": view_checkpoint.name,
        "checkpoint_sha256": digest(view_checkpoint), "raw_sha256": {raw.name: digest(raw) for raw in view_checkpoint_raw},
        "paired_rust_report": "views-report.json", "paired_rust_report_sha256": digest(destination / "views-report.json"),
        "scope": "Historical 84-case checkpoint predates subscription/formatter follow-ups. Isolated owned-scene results use their own exact committed baseline, twenty Rust-only cases and all intermediate regressions; no upstream/module parity inferred."}
    report_bytes = (json.dumps(metadata, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    lines = ["# Foundation performance evidence", "",
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
              "Colored full-frame cases force repaint of twenty 80-cell ASCII rows on an 80x24 frame. Four output traces independently check glyphs, palette/RGB colors, background erasure and hidden cursor. RGB inputs deliberately choose matching profile outputs; they do not compare arbitrary nearest-color algorithms.",
              "Explicit color profiles remain deterministic under NO_COLOR=1. A bounded eight-style cache removes repeated palette search without heap/global state. Retained intermediate samples include truecolor/none regressions, corrected by direct level fast paths; before/after distributions and frame bytes are in report.json.",
              "Known-position horizontal cursor moves select shorter commands. The retained baseline records 6 bytes per move, compared with 4 after the change and 6 for pinned Ultraviolet. Vertical moves use absolute addressing: LF can scroll and CUU/CUD can clamp at existing margins. Column-zero absolute addresses omit the default column parameter. Matea's colored traces additionally replay with nondefault vertical margins; benchmark comparisons use default margins in both.",
              "Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.",
              "Projectile tests add 45 XYZ trajectories. Timed raw/checked updates and construction share the same finite fixture; checks remain enabled in both checked harnesses.",
              "The motion/pagination baseline preserves source hashes and all samples before inlining, independent finite checks, invariant-based ranges and exact dot capacity reservation; see report.json.",
              "Text-input fixtures exclude View construction, clipboard I/O, validators and virtual-cursor commands in both harnesses. They include eight event pairs; Unicode rune-vs-grapheme single-step behavior is not compared.",
              "The text-input baseline retains samples/source hashes before ASCII query-buffer reuse. Unicode filtering keeps contextual lowercase semantics; that path is not covered by the ASCII allocation claim.",
              "Key bindings add three custom-token matches plus construction and metadata access. Existing-string custom tokens do not format/allocate in either language. Constructors retain their result through black_box/global storage; Go escape analysis and Rust ownership differ. Short help storage is inline after the retained baseline.",
              "Four help fixtures render fresh unstyled strings from existing metadata, excluding binding construction and structured-view/cell encoding in both. Ordinary outputs match byte-for-byte; narrow-width overflow and terminal-control safety deviations have separate regressions. Rich Lip Gloss style decorators and whole help-module coverage remain pending.",
              "Clock fixtures cover six fresh compact duration strings, six caller-owned buffer reuse cases and four ignored-event guards. Go setup timers are drained outside timing. Go active Update arms a timer; Rust exposes a reusable schedule descriptor, so isolated active-update comparisons would be mismatched. End-to-end scheduling and full clock-module performance remain pending. Rust allocation bytes are requested sizes; Go includes rounding/tiny-object accounting.",
              "Spinner fixtures cover twelve unstyled preset frame strings and two ignored-ID/generation guards. Both may share immutable frame storage; Rust exposes borrowed Cow output. Labels, rich styles, custom construction, active timer creation and notification latency remain outside this finite comparison.",
              "Raw spring samples use IEEE arithmetic; checked-update cases validate inputs/results in both languages. Rust constructor validation is included. Very short timings are sensitive to harness effects.", "",
              "## Reproduction", "", "```sh", "cargo build --release -p matea-perf-probe --locked",
              "python3 scripts/prepare_upstream_bench.py --references ../references --destination /tmp/matea-foundation-upstream",
              "# In the disposable Go module: go mod tidy; go build -o probe .",
              "MATEA_PERF_ITERATIONS=1000 target/release/matea-perf-probe > artifacts/performance/matea-current.jsonl",
              "MATEA_PERF_ITERATIONS=1000 /tmp/matea-foundation-upstream/probe > artifacts/performance/upstream-current.jsonl",
              "target/release/matea-perf-probe --trace > artifacts/performance/matea-trace.json",
              "/tmp/matea-foundation-upstream/probe --trace > artifacts/performance/ultraviolet-trace.json",
              "target/release/matea-perf-probe --cursor-trace > artifacts/performance/matea-cursor-trace.json",
              "/tmp/matea-foundation-upstream/probe --cursor-trace > artifacts/performance/ultraviolet-cursor-trace.json",
              "NO_COLOR=1 target/release/matea-perf-probe --color-trace > artifacts/performance/matea-color-trace.json",
              "/tmp/matea-foundation-upstream/probe --color-trace > artifacts/performance/ultraviolet-color-trace.json",
              "python3 scripts/check_terminal_trace.py artifacts/performance/matea-trace.json artifacts/performance/ultraviolet-trace.json artifacts/performance/matea-cursor-trace.json artifacts/performance/ultraviolet-cursor-trace.json",
              "python3 scripts/check_color_trace.py artifacts/performance/*-color-trace.json",
              "python3 scripts/report_performance.py --artifacts artifacts/performance", "```", "",
              "Keep reference checkouts clean and at docs/upstream-lock.json revisions. The preparation script verifies pins.",
              "No image processing, GPU or real terminal performance claim is made."]
    lines += ["", "Additional [Rust-only subscription before/after costs](subscription-performance.md) measure adapter construction and retained timer tables, separately from the matched source fixtures."]
    lines += ["", "The [clock-format follow-up](clock-format-performance.md) repeats the existing sixteen clock cases with longer batches and retains all short runs; these overlap the matched case set."]
    lines += ["", "[Owned scene Text costs](view-performance.md) add twenty Rust-only construction/clone/complete-render comparisons, with early-normalization tradeoffs and retained regressions. These are separate from the matched source cases."]
    return {**incoming, "report.json": report_bytes}, ("\n".join(lines) + "\n").encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--matea-binary", type=Path)
    parser.add_argument("--upstream-binary", type=Path)
    parser.add_argument("--check", action="store_true", help="validate/generate in memory without publishing")
    args = parser.parse_args()
    destination = ROOT / "docs/evidence/performance/2026-10-16"
    files, document = generate(args.artifacts, args.matea_binary, args.upstream_binary, destination)
    if args.check:
        print("84 retained cases validated without publication; no new measurement")
        return
    # No invalid record/trace can replace published raw evidence. Individual
    # files are atomic; process interruption during publication is not a
    # multi-file transaction, as documented for compiler evidence imports.
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".matea-performance-", dir=destination.parent) as temporary:
        stage = Path(temporary)
        for name, value in files.items():
            (stage / name).write_bytes(value)
        for name in files:
            os.replace(stage / name, destination / name)
        (stage / "performance.md").write_bytes(document)
        os.replace(stage / "performance.md", ROOT / "docs/performance.md")
    print("Matched fixtures validated before publication; full module parity remains pending")


if __name__ == "__main__":
    main()
