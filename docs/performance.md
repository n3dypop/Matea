# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 3.86 / 3.96 | 28.05 / 40.88 | 0 / 1 |
| input/binding_f12 | 4.34 / 5.68 | 30.40 / 33.63 | 0 / 1 |
| input/binding_miss | 4.63 / 4.81 | 27.25 / 40.41 | 0 / 1 |
| input/binding_disabled | 1.31 / 1.33 | 27.96 / 38.80 | 0 / 1 |
| input/binding_repeat | 3.26 / 3.27 | 27.17 / 40.59 | 0 / 1 |
| input/binding_release | 1.41 / 1.52 | 2.03 / 2.04 | 0 / 0 |
| binding/value_hit | 4.09 / 4.43 | 6.77 / 6.92 | 0 / 0 |
| binding/value_miss | 3.58 / 3.72 | 6.64 / 7.66 | 0 / 0 |
| binding/value_disabled | 0.77 / 0.77 | 6.89 / 7.30 | 0 / 0 |
| binding/construct | 36.42 / 44.22 | 44.54 / 51.22 | 1 / 2 |
| binding/value_hit_owned | 6.03 / 6.17 | 6.96 / 7.28 | 0 / 0 |
| binding/metadata | 1.24 / 1.25 | 2.03 / 2.37 | 0 / 0 |
| motion/frame_duration | 0.71 / 0.87 | 2.09 / 2.17 | 0 / 0 |
| motion/projectile_update | 2.44 / 2.50 | 5.49 / 5.70 | 0 / 0 |
| motion/projectile_checked_update | 2.79 / 2.93 | 6.12 / 6.30 | 0 / 0 |
| motion/projectile_construct | 7.21 / 8.03 | 19.38 / 21.31 | 0 / 1 |
| pagination/range | 0.94 / 0.96 | 0.95 / 1.00 | 0 / 0 |
| pagination/navigation | 1.76 / 1.95 | 5.29 / 5.42 | 0 / 0 |
| pagination/arabic | 50.50 / 55.25 | 66.33 / 71.00 | 1 / 1 |
| pagination/dots_32 | 137.75 / 147.04 | 733.12 / 805.62 | 1 / 31 |
| pagination/dots_256 | 909.17 / 1011.21 | 12936.54 / 13574.17 | 1 / 255 |
| pagination/key_dispatch | 8.12 / 9.45 | 147.51 / 168.33 | 0 / 3 |
| textinput/left_right_ascii_64 | 36.83 / 37.32 | 2741.59 / 2777.39 | 0 / 13 |
| textinput/insert_end_ascii_64 | 68.67 / 73.00 | 2880.99 / 3011.92 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 75.49 / 80.54 | 2919.23 / 2948.40 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 118.56 / 121.91 | 2777.72 / 2803.72 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 92.03 / 97.90 | 3040.65 / 3081.62 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.70 / 4.72 | 2316.38 / 2338.66 | 0 / 1 |
| textinput/word_roundtrip_unicode | 156.21 / 168.40 | 2331.49 / 2396.88 | 0 / 16 |
| textinput/suggestions_edit_pair | 111.12 / 115.98 | 3033.77 / 3136.55 | 0 / 18 |
| cursor/visible_move_80x24 | 27.50 / 27.62 | 42684.38 / 44450.38 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 28671.66 / 32164.16 | 351848.32 / 430865.00 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 28780.84 / 29974.16 | 352928.32 / 397525.82 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 24939.16 / 25890.00 | 134913.32 / 258633.32 | 0 / 549 |
| terminal/colored_full_none_80x24 | 23140.00 / 24106.66 | 122350.84 / 144911.68 | 0 / 349 |
| help/short_unbounded | 302.75 / 312.79 | 11242.00 / 11533.21 | 1 / 23 |
| help/short_40 | 261.12 / 286.67 | 8653.21 / 8879.25 | 1 / 18 |
| help/full_40 | 384.79 / 412.83 | 11102.88 / 11283.12 | 5 / 62 |
| help/short_unicode_80 | 511.92 / 536.75 | 6156.42 / 6308.00 | 1 / 12 |
| render/empty_80x24 | 6212.54 / 6689.17 | 70797.08 / 71531.75 | 1 / 54 |
| render/ascii_80x24 | 13442.25 / 13583.83 | 109107.67 / 111433.83 | 1 / 54 |
| render/unicode_80x24 | 13919.92 / 14097.75 | 86687.04 / 87882.54 | 1 / 54 |
| unicode/width | 663.55 / 685.23 | 3465.49 / 3488.47 | 0 / 0 |
| terminal/unchanged_80x24 | 4.54 / 4.96 | 42930.75 / 43818.38 | 0 / 0 |
| terminal/one_cell_80x24 | 9717.04 / 9882.71 | 44553.54 / 44847.29 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.82 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.11 | 3.02 / 3.12 | 0 / 0 |
| motion/spring_construct_0.2 | 16.80 / 17.79 | 27.42 / 28.43 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.96 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_1 | 2.15 / 2.15 | 3.02 / 3.12 | 0 / 0 |
| motion/spring_construct_1 | 11.68 / 12.20 | 13.84 / 15.65 | 0 / 0 |
| motion/spring_update_2 | 0.84 / 0.84 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_2 | 2.15 / 2.15 | 3.02 / 3.07 | 0 / 0 |
| motion/spring_construct_2 | 17.18 / 17.75 | 25.58 / 63.42 | 0 / 0 |

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
