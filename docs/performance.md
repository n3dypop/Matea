# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 5.41 / 5.91 | 29.29 / 39.85 | 0 / 1 |
| input/binding_f12 | 6.16 / 7.00 | 29.70 / 33.53 | 0 / 1 |
| input/binding_miss | 6.23 / 6.33 | 25.98 / 39.69 | 0 / 1 |
| input/binding_disabled | 1.76 / 1.78 | 28.88 / 38.61 | 0 / 1 |
| input/binding_repeat | 4.32 / 4.33 | 27.93 / 39.90 | 0 / 1 |
| input/binding_release | 1.84 / 1.84 | 2.10 / 2.12 | 0 / 0 |
| binding/value_hit | 5.34 / 5.37 | 6.98 / 7.65 | 0 / 0 |
| binding/value_miss | 4.67 / 4.68 | 6.68 / 7.02 | 0 / 0 |
| binding/value_disabled | 0.87 / 0.88 | 6.80 / 6.94 | 0 / 0 |
| binding/construct | 43.31 / 54.81 | 45.96 / 56.91 | 1 / 2 |
| binding/value_hit_owned | 6.50 / 6.50 | 7.00 / 7.54 | 0 / 0 |
| binding/metadata | 1.34 / 1.35 | 2.10 / 2.18 | 0 / 0 |
| motion/frame_duration | 0.71 / 0.77 | 2.13 / 2.20 | 0 / 0 |
| motion/projectile_update | 2.48 / 2.58 | 5.46 / 5.68 | 0 / 0 |
| motion/projectile_checked_update | 2.80 / 3.11 | 6.14 / 6.35 | 0 / 0 |
| motion/projectile_construct | 7.53 / 8.66 | 20.31 / 22.25 | 0 / 1 |
| pagination/range | 0.94 / 0.95 | 0.94 / 0.98 | 0 / 0 |
| pagination/navigation | 1.75 / 1.98 | 5.34 / 5.54 | 0 / 0 |
| pagination/arabic | 49.21 / 57.00 | 67.00 / 70.04 | 1 / 1 |
| pagination/dots_32 | 137.67 / 150.92 | 751.08 / 859.21 | 1 / 31 |
| pagination/dots_256 | 909.21 / 970.58 | 13367.83 / 13530.29 | 1 / 255 |
| pagination/key_dispatch | 9.15 / 10.09 | 149.79 / 164.68 | 0 / 3 |
| textinput/left_right_ascii_64 | 35.88 / 37.27 | 2747.20 / 2777.01 | 0 / 13 |
| textinput/insert_end_ascii_64 | 69.24 / 71.99 | 2875.87 / 2930.42 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 75.52 / 82.53 | 2929.98 / 2969.40 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 116.11 / 122.44 | 2806.30 / 2944.05 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 91.95 / 95.66 | 3082.02 / 3118.31 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.67 / 4.85 | 2331.04 / 2358.42 | 0 / 1 |
| textinput/word_roundtrip_unicode | 154.83 / 162.43 | 2314.07 / 2348.05 | 0 / 16 |
| textinput/suggestions_edit_pair | 111.63 / 116.35 | 3018.79 / 3065.17 | 0 / 18 |
| cursor/visible_move_80x24 | 27.50 / 27.62 | 42479.17 / 42799.04 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 29279.18 / 36566.66 | 353371.68 / 385704.16 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 28761.66 / 29892.50 | 354726.66 / 386627.50 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 24622.50 / 25502.50 | 135874.18 / 214095.84 | 0 / 549 |
| terminal/colored_full_none_80x24 | 24365.00 / 25660.00 | 122745.82 / 226416.68 | 0 / 349 |
| help/short_unbounded | 285.17 / 298.96 | 11548.88 / 12247.17 | 1 / 23 |
| help/short_40 | 245.83 / 275.54 | 9224.67 / 10300.54 | 1 / 18 |
| help/full_40 | 384.88 / 413.42 | 12485.83 / 13026.08 | 5 / 62 |
| help/short_unicode_80 | 468.50 / 498.83 | 6497.21 / 6634.25 | 1 / 12 |
| clocks/format_zero | 22.62 / 22.67 | 10.62 / 11.83 | 1 / 1 |
| clocks/reused_format_zero | 7.25 / 7.33 | 7.46 / 7.50 | 0 / 0 |
| clocks/format_nanoseconds | 31.79 / 33.08 | 15.96 / 16.75 | 1 / 1 |
| clocks/reused_format_nanoseconds | 16.88 / 16.92 | 10.67 / 10.71 | 0 / 0 |
| clocks/format_microseconds | 38.67 / 38.79 | 17.08 / 19.33 | 1 / 1 |
| clocks/reused_format_microseconds | 25.92 / 25.96 | 12.92 / 13.38 | 0 / 0 |
| clocks/format_seconds | 45.54 / 47.08 | 23.54 / 26.83 | 1 / 1 |
| clocks/reused_format_seconds | 27.79 / 30.50 | 16.83 / 17.50 | 0 / 0 |
| clocks/format_hours | 48.12 / 48.54 | 22.33 / 34.92 | 1 / 1 |
| clocks/reused_format_hours | 31.88 / 31.92 | 18.25 / 28.04 | 0 / 0 |
| clocks/format_maximum_go | 54.42 / 54.54 | 32.08 / 33.17 | 1 / 1 |
| clocks/reused_format_maximum_go | 39.38 / 39.46 | 27.50 / 27.54 | 0 / 0 |
| clocks/stopwatch_paused | 1.25 / 1.26 | 6.33 / 7.32 | 0 / 0 |
| clocks/countdown_paused | 1.57 / 1.57 | 6.66 / 6.93 | 0 / 0 |
| clocks/stopwatch_foreign | 1.25 / 1.26 | 6.35 / 6.37 | 0 / 0 |
| clocks/countdown_foreign | 1.57 / 1.57 | 6.70 / 6.71 | 0 / 0 |
| render/empty_80x24 | 6207.71 / 6315.83 | 73803.67 / 76536.12 | 1 / 54 |
| render/ascii_80x24 | 13360.42 / 13466.92 | 115715.67 / 126562.50 | 1 / 54 |
| render/unicode_80x24 | 13925.54 / 14035.71 | 90423.58 / 96188.17 | 1 / 54 |
| unicode/width | 663.96 / 678.19 | 3650.92 / 3666.35 | 0 / 0 |
| terminal/unchanged_80x24 | 4.96 / 5.21 | 43637.46 / 45118.75 | 0 / 0 |
| terminal/one_cell_80x24 | 9717.42 / 9786.00 | 45163.00 / 45411.71 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 1.16 | 1.33 / 1.40 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.18 / 2.67 | 3.07 / 3.17 | 0 / 0 |
| motion/spring_construct_0.2 | 16.80 / 17.45 | 27.78 / 28.48 | 0 / 0 |
| motion/spring_update_1 | 0.85 / 0.88 | 1.33 / 1.34 | 0 / 0 |
| motion/spring_checked_update_1 | 2.11 / 2.26 | 3.07 / 3.16 | 0 / 0 |
| motion/spring_construct_1 | 11.49 / 12.22 | 13.84 / 14.59 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.82 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_2 | 2.11 / 2.76 | 3.02 / 3.17 | 0 / 0 |
| motion/spring_construct_2 | 16.98 / 17.77 | 49.45 / 62.29 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
Input cases use short batches and can show scheduling outliers; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
The native-cursor fixture also reuses unchanged cells. Four independently replayed states check cell contents, position and visibility; shape/color and terminal I/O are excluded.
Colored full-frame cases force repaint of twenty 80-cell ASCII rows on an 80x24 frame. Four output traces independently check glyphs, palette/RGB colors, background erasure and hidden cursor. RGB inputs deliberately choose matching profile outputs; they do not compare arbitrary nearest-color algorithms.
Explicit color profiles remain deterministic under NO_COLOR=1. A bounded eight-style cache removes repeated palette search without heap/global state. Retained intermediate samples include truecolor/none regressions, corrected by direct level fast paths; before/after distributions and frame bytes are in report.json.
Known-position horizontal cursor moves select shorter commands. The retained baseline records 6 bytes per move, compared with 4 after the change and 6 for pinned Ultraviolet. Vertical moves use absolute addressing: LF can scroll and CUU/CUD can clamp at existing margins. Column-zero absolute addresses omit the default column parameter. Matea's colored traces additionally replay with nondefault vertical margins; benchmark comparisons use default margins in both.
Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.
Projectile tests add 45 XYZ trajectories. Timed raw/checked updates and construction share the same finite fixture; checks remain enabled in both checked harnesses.
The motion/pagination baseline preserves source hashes and all samples before inlining, independent finite checks, invariant-based ranges and exact dot capacity reservation; see report.json.
Text-input fixtures exclude View construction, clipboard I/O, validators and virtual-cursor commands in both harnesses. They include eight event pairs; Unicode rune-vs-grapheme single-step behavior is not compared.
The text-input baseline retains samples/source hashes before ASCII query-buffer reuse. Unicode filtering keeps contextual lowercase semantics; that path is not covered by the ASCII allocation claim.
Key bindings add three custom-token matches plus construction and metadata access. Existing-string custom tokens do not format/allocate in either language. Constructors retain their result through black_box/global storage; Go escape analysis and Rust ownership differ. Short help storage is inline after the retained baseline.
Four help fixtures render fresh unstyled strings from existing metadata, excluding binding construction and structured-view/cell encoding in both. Ordinary outputs match byte-for-byte; narrow-width overflow and terminal-control safety deviations have separate regressions. Rich Lip Gloss style decorators and whole help-module coverage remain pending.
Clock fixtures cover six fresh compact duration strings, six caller-owned buffer reuse cases and four ignored-event guards. Go setup timers are drained outside timing. Go active Update arms a timer; Rust exposes a reusable schedule descriptor, so isolated active-update comparisons would be mismatched. End-to-end scheduling and full clock-module performance remain pending. Rust allocation bytes are requested sizes; Go includes rounding/tiny-object accounting.
Raw spring samples use IEEE arithmetic; checked-update cases validate inputs/results in both languages. Rust constructor validation is included. Very short timings are sensitive to harness effects.

## Reproduction

```sh
cargo build --release -p matea-perf-probe --locked
python3 scripts/prepare_upstream_bench.py --references ../references --destination /tmp/matea-foundation-upstream
# In the disposable Go module: go mod tidy; go build -o probe .
MATEA_PERF_ITERATIONS=1000 target/release/matea-perf-probe > artifacts/performance/matea-current.jsonl
MATEA_PERF_ITERATIONS=1000 /tmp/matea-foundation-upstream/probe > artifacts/performance/upstream-current.jsonl
target/release/matea-perf-probe --trace > artifacts/performance/matea-trace.json
/tmp/matea-foundation-upstream/probe --trace > artifacts/performance/ultraviolet-trace.json
target/release/matea-perf-probe --cursor-trace > artifacts/performance/matea-cursor-trace.json
/tmp/matea-foundation-upstream/probe --cursor-trace > artifacts/performance/ultraviolet-cursor-trace.json
NO_COLOR=1 target/release/matea-perf-probe --color-trace > artifacts/performance/matea-color-trace.json
/tmp/matea-foundation-upstream/probe --color-trace > artifacts/performance/ultraviolet-color-trace.json
python3 scripts/check_terminal_trace.py artifacts/performance/matea-trace.json artifacts/performance/ultraviolet-trace.json artifacts/performance/matea-cursor-trace.json artifacts/performance/ultraviolet-cursor-trace.json
python3 scripts/check_color_trace.py artifacts/performance/*-color-trace.json
python3 scripts/report_performance.py --artifacts artifacts/performance
```

Keep reference checkouts clean and at docs/upstream-lock.json revisions. The preparation script verifies pins.
No image processing, GPU or real terminal performance claim is made.
