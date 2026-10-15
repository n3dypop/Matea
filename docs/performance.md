# Foundation performance evidence — 2026-10-16

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| render/empty_80x24 | 6300.88 / 7078.38 | 68368.83 / 83814.12 | 1 / 54 |
| render/ascii_80x24 | 13374.17 / 13678.42 | 106237.21 / 107438.88 | 1 / 54 |
| render/unicode_80x24 | 13945.83 / 14165.58 | 84213.83 / 85631.58 | 1 / 54 |
| unicode/width | 699.05 / 726.37 | 3481.75 / 3523.20 | 0 / 0 |
| terminal/unchanged_80x24 | 4.00 / 16.75 | 42548.12 / 42941.92 | 0 / 0 |
| terminal/one_cell_80x24 | 10160.96 / 13901.42 | 44236.38 / 45294.46 | 0 / 5 |
| motion/spring_update_0.2 | 2.15 / 2.20 | 1.29 / 1.77 | 0 / 0 |
| motion/spring_construct_0.2 | 16.79 / 17.75 | 26.83 / 28.77 | 0 / 0 |
| motion/spring_update_1 | 2.12 / 2.48 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_construct_1 | 11.48 / 11.72 | 13.67 / 14.66 | 0 / 0 |
| motion/spring_update_2 | 2.11 / 2.15 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_construct_2 | 16.75 / 17.51 | 48.31 / 53.76 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.
Rust rejects invalid/nonfinite motion input, an extra cost included here. Very short timings are sensitive to harness effects.

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
