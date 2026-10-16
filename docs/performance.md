# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 5.40 / 5.44 | 28.33 / 35.66 | 0 / 1 |
| input/binding_f12 | 5.07 / 5.44 | 29.99 / 43.02 | 0 / 1 |
| input/binding_miss | 6.23 / 6.24 | 25.57 / 35.98 | 0 / 1 |
| input/binding_disabled | 1.76 / 1.79 | 27.88 / 39.74 | 0 / 1 |
| input/binding_repeat | 3.81 / 4.68 | 27.64 / 38.29 | 0 / 1 |
| input/binding_release | 1.84 / 1.84 | 2.04 / 2.08 | 0 / 0 |
| binding/value_hit | 5.34 / 5.43 | 6.83 / 7.22 | 0 / 0 |
| binding/value_miss | 4.15 / 4.77 | 6.72 / 6.83 | 0 / 0 |
| binding/value_disabled | 0.87 / 0.87 | 6.77 / 7.77 | 0 / 0 |
| binding/construct | 43.11 / 54.94 | 44.49 / 47.39 | 1 / 2 |
| binding/value_hit_owned | 6.50 / 6.59 | 6.91 / 7.44 | 0 / 0 |
| binding/metadata | 1.34 / 1.34 | 2.10 / 2.19 | 0 / 0 |
| motion/frame_duration | 0.72 / 0.94 | 2.12 / 2.18 | 0 / 0 |
| motion/projectile_update | 2.52 / 2.66 | 5.48 / 5.77 | 0 / 0 |
| motion/projectile_checked_update | 3.11 / 3.38 | 6.17 / 6.29 | 0 / 0 |
| motion/projectile_construct | 7.45 / 8.38 | 20.20 / 21.00 | 0 / 1 |
| pagination/range | 0.94 / 0.96 | 0.96 / 1.01 | 0 / 0 |
| pagination/navigation | 1.76 / 1.95 | 5.37 / 5.52 | 0 / 0 |
| pagination/arabic | 51.58 / 70.17 | 64.17 / 65.21 | 1 / 1 |
| pagination/dots_32 | 137.67 / 142.29 | 745.00 / 881.92 | 1 / 31 |
| pagination/dots_256 | 911.25 / 956.50 | 13123.58 / 13458.25 | 1 / 255 |
| pagination/key_dispatch | 8.26 / 9.46 | 148.46 / 171.94 | 0 / 3 |
| textinput/left_right_ascii_64 | 35.77 / 38.42 | 2734.22 / 2778.37 | 0 / 13 |
| textinput/insert_end_ascii_64 | 68.66 / 71.53 | 2862.90 / 2916.74 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 76.17 / 84.92 | 2912.55 / 2949.61 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 116.35 / 122.46 | 2783.25 / 2814.35 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 91.46 / 95.27 | 3061.88 / 3097.80 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.73 / 4.75 | 2321.61 / 2360.51 | 0 / 1 |
| textinput/word_roundtrip_unicode | 154.93 / 161.59 | 2335.35 / 2377.36 | 0 / 16 |
| textinput/suggestions_edit_pair | 110.14 / 115.54 | 3033.77 / 3100.39 | 0 / 18 |
| cursor/visible_move_80x24 | 28.42 / 28.50 | 42897.92 / 44415.46 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 28492.50 / 30006.66 | 353870.00 / 432820.00 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 29057.50 / 31797.50 | 374706.68 / 506020.00 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 24192.50 / 26664.16 | 135889.16 / 224900.00 | 0 / 549 |
| terminal/colored_full_none_80x24 | 23154.16 / 24212.50 | 123467.50 / 147881.66 | 0 / 349 |
| help/short_unbounded | 283.67 / 303.62 | 11279.25 / 11560.25 | 1 / 23 |
| help/short_40 | 241.50 / 259.21 | 8646.12 / 8829.04 | 1 / 18 |
| help/full_40 | 393.67 / 459.17 | 11061.67 / 11388.50 | 5 / 62 |
| help/short_unicode_80 | 479.29 / 497.71 | 6193.83 / 6350.12 | 1 / 12 |
| render/empty_80x24 | 6219.04 / 6501.17 | 71007.25 / 72530.62 | 1 / 54 |
| render/ascii_80x24 | 13416.67 / 13553.42 | 107407.21 / 111847.79 | 1 / 54 |
| render/unicode_80x24 | 14021.92 / 14614.50 | 85982.21 / 89871.92 | 1 / 54 |
| unicode/width | 664.63 / 691.21 | 3475.47 / 3488.07 | 0 / 0 |
| terminal/unchanged_80x24 | 5.50 / 5.54 | 42942.08 / 43452.21 | 0 / 0 |
| terminal/one_cell_80x24 | 9645.88 / 9818.25 | 44601.62 / 45172.08 | 0 / 5 |
| motion/spring_update_0.2 | 0.88 / 0.88 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.26 / 4.64 | 3.02 / 3.43 | 0 / 0 |
| motion/spring_construct_0.2 | 19.05 / 96.11 | 27.20 / 28.60 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.84 | 1.31 / 1.33 | 0 / 0 |
| motion/spring_checked_update_1 | 2.15 / 2.43 | 3.02 / 3.93 | 0 / 0 |
| motion/spring_construct_1 | 11.85 / 12.03 | 13.83 / 15.44 | 0 / 0 |
| motion/spring_update_2 | 0.84 / 0.88 | 1.32 / 1.32 | 0 / 0 |
| motion/spring_checked_update_2 | 2.15 / 2.21 | 3.07 / 3.09 | 0 / 0 |
| motion/spring_construct_2 | 17.24 / 18.01 | 25.46 / 48.73 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
Input cases use short batches and can show scheduling outliers; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
The native-cursor fixture also reuses unchanged cells. Four independently replayed states check cell contents, position and visibility; shape/color and terminal I/O are excluded.
Colored full-frame cases force repaint of twenty 80-cell ASCII rows on an 80x24 frame. Four output traces independently check glyphs, palette/RGB colors, background erasure and hidden cursor. RGB inputs deliberately choose matching profile outputs; they do not compare arbitrary nearest-color algorithms.
Explicit color profiles remain deterministic under NO_COLOR=1. A bounded eight-style cache removes repeated palette search without heap/global state. Retained intermediate samples include truecolor/none regressions, corrected by direct level fast paths; before/after distributions and frame bytes are in report.json.
Known-position same-axis cursor moves select shorter relative commands. The retained baseline records 6 bytes per move, compared with 4 after the change and 6 for the pinned Ultraviolet public MoveTo fixture. Unknown positions reanchor absolutely. A known next-row start uses CRLF, cancelling delayed wrap first; no row advance is issued below the surface.
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
