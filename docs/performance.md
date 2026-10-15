# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 2.01 / 213.17 | 29.83 / 41.71 | 0 / 1 |
| input/binding_f12 | 2.39 / 48.90 | 29.80 / 33.86 | 0 / 1 |
| input/binding_miss | 3.57 / 4.01 | 25.23 / 38.87 | 0 / 1 |
| input/binding_disabled | 1.30 / 1.70 | 30.73 / 44.48 | 0 / 1 |
| input/binding_repeat | 1.95 / 2.17 | 27.63 / 39.39 | 0 / 1 |
| input/binding_release | 0.97 / 0.97 | 2.11 / 2.18 | 0 / 0 |
| render/empty_80x24 | 6225.00 / 9089.12 | 69605.42 / 70707.00 | 1 / 54 |
| render/ascii_80x24 | 13376.00 / 13604.38 | 107220.12 / 113341.67 | 1 / 54 |
| render/unicode_80x24 | 13859.96 / 14105.00 | 84990.62 / 90235.21 | 1 / 54 |
| unicode/width | 664.41 / 680.83 | 3486.23 / 3574.75 | 0 / 0 |
| terminal/unchanged_80x24 | 5.38 / 5.46 | 42714.00 / 43391.67 | 0 / 0 |
| terminal/one_cell_80x24 | 9866.71 / 10048.54 | 44098.08 / 44605.79 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.82 | 1.31 / 1.39 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.11 | 2.97 / 3.12 | 0 / 0 |
| motion/spring_construct_0.2 | 16.75 / 18.78 | 26.87 / 28.12 | 0 / 0 |
| motion/spring_update_1 | 0.84 / 0.84 | 1.29 / 1.38 | 0 / 0 |
| motion/spring_checked_update_1 | 2.15 / 2.16 | 2.97 / 3.42 | 0 / 0 |
| motion/spring_construct_1 | 11.66 / 12.27 | 13.58 / 14.05 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.82 | 1.33 / 1.34 | 0 / 0 |
| motion/spring_checked_update_2 | 2.11 / 2.11 | 3.07 / 3.15 | 0 / 0 |
| motion/spring_construct_2 | 16.75 / 18.20 | 25.30 / 80.74 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
Input cases use short batches and show scheduling outliers in this run; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.
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
