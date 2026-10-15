# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 4.09 / 4.09 | 29.69 / 50.73 | 0 / 1 |
| input/binding_f12 | 4.06 / 4.67 | 29.58 / 33.27 | 0 / 1 |
| input/binding_miss | 6.37 / 6.38 | 25.29 / 41.37 | 0 / 1 |
| input/binding_disabled | 2.32 / 2.33 | 30.30 / 64.92 | 0 / 1 |
| input/binding_repeat | 3.11 / 3.20 | 27.77 / 39.34 | 0 / 1 |
| input/binding_release | 1.54 / 1.63 | 2.03 / 2.46 | 0 / 0 |
| motion/frame_duration | 0.98 / 1.11 | 2.10 / 2.19 | 0 / 0 |
| motion/projectile_update | 2.96 / 3.53 | 5.52 / 5.82 | 0 / 0 |
| motion/projectile_checked_update | 2.85 / 3.44 | 6.01 / 6.24 | 0 / 0 |
| motion/projectile_construct | 7.47 / 8.06 | 20.09 / 22.68 | 0 / 1 |
| pagination/range | 0.91 / 0.99 | 0.94 / 1.02 | 0 / 0 |
| pagination/navigation | 1.74 / 1.84 | 5.34 / 5.59 | 0 / 0 |
| pagination/arabic | 46.96 / 47.67 | 64.04 / 72.54 | 1 / 1 |
| pagination/dots_32 | 138.17 / 140.62 | 776.83 / 931.00 | 1 / 31 |
| pagination/dots_256 | 912.42 / 964.08 | 13039.96 / 13699.17 | 1 / 255 |
| pagination/key_dispatch | 13.43 / 14.20 | 148.61 / 170.34 | 0 / 3 |
| render/empty_80x24 | 6186.83 / 6331.21 | 69095.58 / 72901.12 | 1 / 54 |
| render/ascii_80x24 | 13381.92 / 13573.17 | 107623.96 / 113069.17 | 1 / 54 |
| render/unicode_80x24 | 13844.29 / 14808.29 | 86206.50 / 90377.58 | 1 / 54 |
| unicode/width | 663.30 / 677.75 | 3464.02 / 3589.25 | 0 / 0 |
| terminal/unchanged_80x24 | 5.38 / 5.46 | 42545.54 / 43326.67 | 0 / 0 |
| terminal/one_cell_80x24 | 9830.71 / 9979.42 | 44156.42 / 44634.92 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.82 | 1.32 / 1.34 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.20 | 3.02 / 3.07 | 0 / 0 |
| motion/spring_construct_0.2 | 16.65 / 17.58 | 27.35 / 29.98 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.82 | 1.31 / 1.35 | 0 / 0 |
| motion/spring_checked_update_1 | 2.11 / 2.31 | 2.97 / 3.02 | 0 / 0 |
| motion/spring_construct_1 | 11.50 / 12.43 | 13.58 / 14.34 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.82 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_checked_update_2 | 2.11 / 2.11 | 2.97 / 3.03 | 0 / 0 |
| motion/spring_construct_2 | 16.90 / 18.45 | 47.61 / 62.48 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
Input cases use short batches and can show scheduling outliers; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.
Projectile tests add 45 XYZ trajectories. Timed raw/checked updates and construction share the same finite fixture; checks remain enabled in both checked harnesses.
The motion/pagination baseline preserves source hashes and all samples before inlining, independent finite checks, invariant-based ranges and exact dot capacity reservation; see report.json.
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
