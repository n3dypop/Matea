# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 1.91 / 5.53 | 59.01 / 85.38 | 0 / 1 |
| input/binding_f12 | 6.32 / 8.94 | 35.05 / 40.67 | 0 / 1 |
| input/binding_miss | 2.50 / 2.90 | 25.99 / 43.86 | 0 / 1 |
| input/binding_disabled | 0.70 / 0.71 | 27.83 / 41.22 | 0 / 1 |
| input/binding_repeat | 1.77 / 2.10 | 27.85 / 39.56 | 0 / 1 |
| input/binding_release | 0.88 / 0.93 | 2.03 / 2.19 | 0 / 0 |
| binding/value_hit | 2.50 / 2.61 | 6.89 / 6.99 | 0 / 0 |
| binding/value_miss | 2.19 / 2.32 | 6.69 / 7.30 | 0 / 0 |
| binding/value_disabled | 0.47 / 0.47 | 7.07 / 8.08 | 0 / 0 |
| binding/construct | 29.43 / 31.52 | 45.30 / 51.73 | 1 / 2 |
| binding/value_hit_owned | 5.62 / 5.77 | 7.10 / 7.98 | 0 / 0 |
| binding/metadata | 1.16 / 1.26 | 2.03 / 2.36 | 0 / 0 |
| motion/frame_duration | 0.68 / 0.69 | 2.08 / 2.20 | 0 / 0 |
| motion/projectile_update | 2.41 / 2.89 | 5.51 / 5.80 | 0 / 0 |
| motion/projectile_checked_update | 2.84 / 2.95 | 6.22 / 6.50 | 0 / 0 |
| motion/projectile_construct | 7.53 / 7.98 | 20.22 / 21.58 | 0 / 1 |
| pagination/range | 0.98 / 1.06 | 0.96 / 1.05 | 0 / 0 |
| pagination/navigation | 1.79 / 1.96 | 5.43 / 5.87 | 0 / 0 |
| pagination/arabic | 49.46 / 51.12 | 64.71 / 65.46 | 1 / 1 |
| pagination/dots_32 | 140.42 / 161.67 | 765.12 / 896.21 | 1 / 31 |
| pagination/dots_256 | 909.88 / 963.17 | 13428.08 / 13750.33 | 1 / 255 |
| pagination/key_dispatch | 8.12 / 9.47 | 151.67 / 170.56 | 0 / 3 |
| textinput/left_right_ascii_64 | 35.10 / 36.49 | 2742.60 / 2780.00 | 0 / 13 |
| textinput/insert_end_ascii_64 | 67.93 / 71.23 | 2855.05 / 2895.35 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 75.79 / 78.43 | 2897.35 / 2926.53 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 116.19 / 121.32 | 2774.51 / 2800.17 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 91.75 / 96.58 | 3058.78 / 3086.20 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.59 / 4.78 | 2334.66 / 2379.85 | 0 / 1 |
| textinput/word_roundtrip_unicode | 154.73 / 160.33 | 2346.66 / 2393.43 | 0 / 16 |
| textinput/suggestions_edit_pair | 111.42 / 115.32 | 3049.62 / 3248.50 | 0 / 18 |
| cursor/visible_move_80x24 | 28.75 / 28.79 | 42608.00 / 42987.71 | 0 / 1 |
| render/empty_80x24 | 6210.21 / 6309.62 | 69891.62 / 70716.08 | 1 / 54 |
| render/ascii_80x24 | 13351.79 / 14087.75 | 107422.33 / 109682.50 | 1 / 54 |
| render/unicode_80x24 | 13953.29 / 14182.29 | 85500.08 / 90307.04 | 1 / 54 |
| unicode/width | 665.52 / 678.41 | 3474.04 / 3491.13 | 0 / 0 |
| terminal/unchanged_80x24 | 5.58 / 5.67 | 42858.04 / 43151.08 | 0 / 0 |
| terminal/one_cell_80x24 | 9860.00 / 9978.29 | 44518.92 / 44816.62 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.98 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.15 / 2.25 | 3.02 / 3.36 | 0 / 0 |
| motion/spring_construct_0.2 | 17.07 / 17.84 | 39.32 / 42.44 | 0 / 0 |
| motion/spring_update_1 | 0.84 / 0.84 | 1.32 / 1.41 | 0 / 0 |
| motion/spring_checked_update_1 | 2.19 / 2.19 | 3.02 / 3.57 | 0 / 0 |
| motion/spring_construct_1 | 11.67 / 12.57 | 13.83 / 14.44 | 0 / 0 |
| motion/spring_update_2 | 0.85 / 0.89 | 1.31 / 1.33 | 0 / 0 |
| motion/spring_checked_update_2 | 2.15 / 2.38 | 3.11 / 3.30 | 0 / 0 |
| motion/spring_construct_2 | 16.80 / 17.69 | 25.97 / 48.16 | 0 / 0 |

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
python3 scripts/check_terminal_trace.py artifacts/performance/*-trace.json
```

Keep reference checkouts clean and at docs/upstream-lock.json revisions. The preparation script verifies pins.
No image processing, GPU or real terminal performance claim is made.
