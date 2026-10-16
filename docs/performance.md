# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 5.84 / 6.26 | 29.64 / 44.07 | 0 / 1 |
| input/binding_f12 | 6.75 / 6.90 | 29.77 / 34.90 | 0 / 1 |
| input/binding_miss | 8.69 / 8.85 | 25.76 / 42.96 | 0 / 1 |
| input/binding_disabled | 2.70 / 3.15 | 30.52 / 83.60 | 0 / 1 |
| input/binding_repeat | 4.02 / 4.09 | 28.64 / 47.14 | 0 / 1 |
| input/binding_release | 2.01 / 2.04 | 2.03 / 2.37 | 0 / 0 |
| motion/frame_duration | 1.24 / 1.43 | 2.11 / 2.36 | 0 / 0 |
| motion/projectile_update | 3.53 / 3.98 | 5.50 / 5.73 | 0 / 0 |
| motion/projectile_checked_update | 3.00 / 3.25 | 6.09 / 6.39 | 0 / 0 |
| motion/projectile_construct | 7.45 / 7.79 | 20.40 / 21.48 | 0 / 1 |
| pagination/range | 0.99 / 1.04 | 0.94 / 0.96 | 0 / 0 |
| pagination/navigation | 1.88 / 1.89 | 5.38 / 5.60 | 0 / 0 |
| pagination/arabic | 48.38 / 49.62 | 64.29 / 70.83 | 1 / 1 |
| pagination/dots_32 | 137.83 / 139.38 | 774.46 / 949.50 | 1 / 31 |
| pagination/dots_256 | 910.88 / 950.75 | 13173.92 / 13729.46 | 1 / 255 |
| pagination/key_dispatch | 13.45 / 14.53 | 153.69 / 198.95 | 0 / 3 |
| textinput/left_right_ascii_64 | 45.58 / 45.84 | 2858.41 / 3272.94 | 0 / 13 |
| textinput/insert_end_ascii_64 | 79.20 / 82.99 | 2881.64 / 2912.22 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 85.05 / 90.26 | 2924.52 / 2949.62 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 129.99 / 136.05 | 2809.54 / 3909.67 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 98.44 / 101.16 | 3072.58 / 3121.16 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.68 / 4.72 | 2329.51 / 2353.70 | 0 / 1 |
| textinput/word_roundtrip_unicode | 174.64 / 181.06 | 2365.13 / 2450.34 | 0 / 16 |
| textinput/suggestions_edit_pair | 121.80 / 125.98 | 3026.49 / 3080.05 | 0 / 18 |
| render/empty_80x24 | 6215.33 / 6376.79 | 69710.46 / 74245.92 | 1 / 54 |
| render/ascii_80x24 | 13356.71 / 13867.21 | 108414.83 / 113676.71 | 1 / 54 |
| render/unicode_80x24 | 14122.54 / 14837.83 | 86486.46 / 90423.38 | 1 / 54 |
| unicode/width | 669.79 / 704.58 | 3451.61 / 3613.72 | 0 / 0 |
| terminal/unchanged_80x24 | 3.58 / 3.62 | 42663.46 / 43116.96 | 0 / 0 |
| terminal/one_cell_80x24 | 9912.50 / 10047.33 | 44207.88 / 44618.08 | 0 / 5 |
| motion/spring_update_0.2 | 0.94 / 0.96 | 1.31 / 1.63 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.19 / 2.19 | 3.02 / 3.93 | 0 / 0 |
| motion/spring_construct_0.2 | 17.07 / 18.12 | 26.91 / 28.50 | 0 / 0 |
| motion/spring_update_1 | 0.96 / 0.96 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_checked_update_1 | 2.19 / 2.20 | 2.97 / 3.16 | 0 / 0 |
| motion/spring_construct_1 | 11.66 / 12.10 | 13.58 / 15.33 | 0 / 0 |
| motion/spring_update_2 | 0.96 / 0.97 | 1.29 / 1.31 | 0 / 0 |
| motion/spring_checked_update_2 | 2.19 / 2.21 | 3.02 / 3.05 | 0 / 0 |
| motion/spring_construct_2 | 17.06 / 17.70 | 47.71 / 61.97 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
Input cases use short batches and can show scheduling outliers; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
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
