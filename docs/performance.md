# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| render/empty_80x24 | 6268.21 / 9614.75 | 68286.67 / 72688.29 | 1 / 54 |
| render/ascii_80x24 | 13277.67 / 13487.21 | 106334.83 / 107969.83 | 1 / 54 |
| render/unicode_80x24 | 13949.12 / 17206.12 | 84200.46 / 85159.33 | 1 / 54 |
| unicode/width | 667.28 / 702.43 | 3465.56 / 3493.40 | 0 / 0 |
| terminal/unchanged_80x24 | 3.62 / 3.67 | 42523.79 / 42923.79 | 0 / 0 |
| terminal/one_cell_80x24 | 9965.12 / 10309.92 | 43987.67 / 44275.33 | 0 / 5 |
| motion/spring_update_0.2 | 0.85 / 0.89 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.28 | 2.97 / 3.08 | 0 / 0 |
| motion/spring_construct_0.2 | 16.87 / 17.80 | 26.84 / 29.12 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.82 | 1.34 / 1.44 | 0 / 0 |
| motion/spring_checked_update_1 | 2.11 / 2.11 | 3.07 / 3.11 | 0 / 0 |
| motion/spring_construct_1 | 11.50 / 11.79 | 13.60 / 14.56 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.88 | 1.30 / 1.37 | 0 / 0 |
| motion/spring_checked_update_2 | 2.14 / 3.14 | 2.99 / 3.08 | 0 / 0 |
| motion/spring_construct_2 | 16.91 / 18.08 | 60.84 / 63.50 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
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
