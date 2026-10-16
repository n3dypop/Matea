# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 4.01 / 4.70 | 27.96 / 41.89 | 0 / 1 |
| input/binding_f12 | 4.67 / 5.09 | 29.71 / 33.67 | 0 / 1 |
| input/binding_miss | 6.37 / 7.34 | 26.00 / 43.81 | 0 / 1 |
| input/binding_disabled | 2.32 / 3.70 | 28.10 / 40.69 | 0 / 1 |
| input/binding_repeat | 3.48 / 3.57 | 29.68 / 43.68 | 0 / 1 |
| input/binding_release | 1.54 / 1.69 | 2.18 / 2.20 | 0 / 0 |
| motion/frame_duration | 1.09 / 1.12 | 2.24 / 2.32 | 0 / 0 |
| motion/projectile_update | 2.98 / 3.53 | 5.60 / 6.13 | 0 / 0 |
| motion/projectile_checked_update | 2.98 / 3.17 | 6.17 / 6.31 | 0 / 0 |
| motion/projectile_construct | 7.46 / 8.07 | 20.20 / 23.59 | 0 / 1 |
| pagination/range | 0.98 / 1.04 | 0.97 / 1.27 | 0 / 0 |
| pagination/navigation | 1.78 / 1.96 | 5.44 / 5.61 | 0 / 0 |
| pagination/arabic | 47.46 / 47.54 | 65.54 / 66.50 | 1 / 1 |
| pagination/dots_32 | 137.83 / 139.62 | 768.38 / 835.25 | 1 / 31 |
| pagination/dots_256 | 910.12 / 973.83 | 12931.42 / 13369.25 | 1 / 255 |
| pagination/key_dispatch | 13.45 / 15.36 | 147.65 / 171.05 | 0 / 3 |
| textinput/left_right_ascii_64 | 45.09 / 46.86 | 2757.89 / 2857.08 | 0 / 13 |
| textinput/insert_end_ascii_64 | 78.02 / 89.12 | 2887.95 / 2940.12 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 85.18 / 89.40 | 2929.09 / 2965.05 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 129.18 / 138.72 | 2793.25 / 2822.25 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 100.14 / 105.62 | 3080.28 / 3120.98 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.60 / 4.87 | 2339.23 / 2359.43 | 0 / 1 |
| textinput/word_roundtrip_unicode | 175.56 / 185.22 | 2354.13 / 2435.74 | 0 / 16 |
| textinput/suggestions_edit_pair | 123.25 / 130.20 | 3063.96 / 3126.20 | 0 / 18 |
| cursor/visible_move_80x24 | 27.50 / 27.92 | 42609.29 / 43674.46 | 0 / 1 |
| render/empty_80x24 | 6452.67 / 6858.71 | 68258.67 / 71811.83 | 1 / 54 |
| render/ascii_80x24 | 13369.08 / 13689.62 | 107293.75 / 108247.08 | 1 / 54 |
| render/unicode_80x24 | 13945.42 / 14046.25 | 83776.21 / 85845.04 | 1 / 54 |
| unicode/width | 664.58 / 672.15 | 3469.64 / 3496.37 | 0 / 0 |
| terminal/unchanged_80x24 | 4.42 / 4.46 | 42579.67 / 42899.29 | 0 / 0 |
| terminal/one_cell_80x24 | 9893.04 / 9967.75 | 44350.12 / 44558.25 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.88 | 1.31 / 1.99 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.15 / 2.23 | 2.97 / 3.03 | 0 / 0 |
| motion/spring_construct_0.2 | 16.75 / 17.34 | 26.88 / 27.42 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.84 | 1.31 / 1.40 | 0 / 0 |
| motion/spring_checked_update_1 | 2.15 / 2.19 | 3.02 / 3.36 | 0 / 0 |
| motion/spring_construct_1 | 11.46 / 11.98 | 13.84 / 14.16 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.84 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_2 | 2.26 / 2.71 | 3.02 / 3.23 | 0 / 0 |
| motion/spring_construct_2 | 16.75 / 17.64 | 25.50 / 49.37 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
Input cases use short batches and can show scheduling outliers; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
The native-cursor fixture also reuses unchanged cells. Four independently replayed states check cell contents, position and visibility; shape/color and terminal I/O are excluded.
Known-position same-axis cursor moves now select shorter relative commands. The retained baseline records 6 bytes per move, compared with 4 after the change and 6 for the pinned Ultraviolet public MoveTo fixture. Full repaint reanchors absolutely; wrap-pending positions never use this shortcut.
Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.
Projectile tests add 45 XYZ trajectories. Timed raw/checked updates and construction share the same finite fixture; checks remain enabled in both checked harnesses.
The motion/pagination baseline preserves source hashes and all samples before inlining, independent finite checks, invariant-based ranges and exact dot capacity reservation; see report.json.
Text-input fixtures exclude View construction, clipboard I/O, validators and virtual-cursor commands in both harnesses. They include eight event pairs; Unicode rune-vs-grapheme single-step behavior is not compared.
The text-input baseline retains samples/source hashes before ASCII query-buffer reuse. Unicode filtering keeps contextual lowercase semantics; that path is not covered by the ASCII allocation claim.
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
python3 scripts/check_terminal_trace.py artifacts/performance/*-trace.json
```

Keep reference checkouts clean and at docs/upstream-lock.json revisions. The preparation script verifies pins.
No image processing, GPU or real terminal performance claim is made.
