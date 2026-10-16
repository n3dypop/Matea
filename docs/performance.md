# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 2.92 / 2.92 | 28.25 / 41.58 | 0 / 1 |
| input/binding_f12 | 3.33 / 3.55 | 30.31 / 34.02 | 0 / 1 |
| input/binding_miss | 3.67 / 3.69 | 25.32 / 40.25 | 0 / 1 |
| input/binding_disabled | 1.03 / 1.12 | 28.87 / 40.20 | 0 / 1 |
| input/binding_repeat | 2.54 / 2.54 | 29.41 / 211.99 | 0 / 1 |
| input/binding_release | 1.26 / 1.26 | 2.07 / 2.16 | 0 / 0 |
| binding/value_hit | 3.35 / 3.58 | 6.87 / 7.76 | 0 / 0 |
| binding/value_miss | 2.93 / 3.18 | 6.71 / 6.98 | 0 / 0 |
| binding/value_disabled | 0.63 / 0.63 | 6.76 / 7.09 | 0 / 0 |
| binding/construct | 34.09 / 40.20 | 46.60 / 49.07 | 1 / 2 |
| binding/value_hit_owned | 6.03 / 6.96 | 6.98 / 7.59 | 0 / 0 |
| binding/metadata | 1.20 / 1.30 | 2.14 / 2.52 | 0 / 0 |
| motion/frame_duration | 0.69 / 0.72 | 2.09 / 2.19 | 0 / 0 |
| motion/projectile_update | 2.56 / 2.77 | 5.48 / 5.70 | 0 / 0 |
| motion/projectile_checked_update | 3.26 / 3.58 | 6.14 / 6.32 | 0 / 0 |
| motion/projectile_construct | 7.03 / 8.54 | 20.32 / 21.31 | 0 / 1 |
| pagination/range | 0.94 / 0.97 | 0.95 / 0.96 | 0 / 0 |
| pagination/navigation | 1.75 / 1.94 | 5.36 / 5.58 | 0 / 0 |
| pagination/arabic | 48.25 / 52.92 | 65.79 / 70.62 | 1 / 1 |
| pagination/dots_32 | 138.58 / 138.71 | 770.62 / 862.83 | 1 / 31 |
| pagination/dots_256 | 911.08 / 972.33 | 13471.33 / 13721.25 | 1 / 255 |
| pagination/key_dispatch | 8.52 / 9.65 | 149.07 / 173.11 | 0 / 3 |
| textinput/left_right_ascii_64 | 35.23 / 36.88 | 2746.17 / 2773.98 | 0 / 13 |
| textinput/insert_end_ascii_64 | 67.69 / 72.14 | 2883.75 / 2924.95 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 76.15 / 80.68 | 2934.71 / 2959.61 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 116.17 / 120.77 | 2798.12 / 2848.68 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 91.10 / 96.93 | 3087.72 / 3121.41 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.59 / 4.60 | 2336.45 / 2362.10 | 0 / 1 |
| textinput/word_roundtrip_unicode | 155.45 / 163.38 | 2323.85 / 2391.88 | 0 / 16 |
| textinput/suggestions_edit_pair | 111.00 / 120.60 | 3029.80 / 3088.62 | 0 / 18 |
| cursor/visible_move_80x24 | 28.58 / 28.62 | 42740.71 / 43060.29 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 28615.00 / 30347.50 | 351630.00 / 432174.16 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 28472.50 / 31150.00 | 354086.66 / 383485.84 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 24580.82 / 25429.18 | 134543.32 / 208110.84 | 0 / 549 |
| terminal/colored_full_none_80x24 | 23352.50 / 25080.84 | 122332.50 / 125963.34 | 0 / 349 |
| render/empty_80x24 | 6236.00 / 6708.54 | 70300.71 / 71422.54 | 1 / 54 |
| render/ascii_80x24 | 13357.12 / 13649.75 | 108835.50 / 122752.88 | 1 / 54 |
| render/unicode_80x24 | 13962.25 / 14083.12 | 86298.38 / 89339.38 | 1 / 54 |
| unicode/width | 664.08 / 676.98 | 3469.85 / 3502.76 | 0 / 0 |
| terminal/unchanged_80x24 | 5.50 / 5.54 | 42815.83 / 42973.29 | 0 / 0 |
| terminal/one_cell_80x24 | 9628.12 / 9793.42 | 44538.12 / 44738.17 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.82 | 1.31 / 1.62 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.20 | 3.02 / 3.14 | 0 / 0 |
| motion/spring_construct_0.2 | 17.00 / 17.62 | 27.44 / 28.72 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.85 | 1.32 / 1.75 | 0 / 0 |
| motion/spring_checked_update_1 | 2.11 / 2.11 | 3.02 / 3.02 | 0 / 0 |
| motion/spring_construct_1 | 11.54 / 12.91 | 13.83 / 14.00 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.82 | 1.38 / 1.93 | 0 / 0 |
| motion/spring_checked_update_2 | 2.15 / 2.16 | 3.02 / 3.06 | 0 / 0 |
| motion/spring_construct_2 | 17.10 / 17.82 | 25.54 / 63.53 | 0 / 0 |

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
