# Clock formatting follow-up

Same sixteen clock workloads; a follow-up of the existing 84-case set, not sixteen new capabilities.
Baseline commit: `ac6fb99fac63d61ca3fcc25869b6d32965905b33`. [Raw runs, hashes, compilers and limits](evidence/performance/2026-10-16/clocks-reverse-report.json).

| Workload | forward p50 / p95 ns | reverse p50 / p95 ns | Go after p50 / p95 ns | allocations M / Go |
| --- | ---: | ---: | ---: | ---: |
| clocks/format_zero | 30.34 / 33.58 | 26.17 / 29.02 | 10.54 / 11.25 | 1 / 1 |
| clocks/reused_format_zero | 8.30 / 10.86 | 7.71 / 7.85 | 7.18 / 7.27 | 0 / 0 |
| clocks/format_nanoseconds | 34.12 / 37.07 | 32.03 / 33.43 | 15.56 / 21.73 | 1 / 1 |
| clocks/reused_format_nanoseconds | 17.12 / 17.80 | 15.05 / 15.52 | 10.30 / 10.41 | 0 / 0 |
| clocks/format_microseconds | 41.44 / 50.37 | 35.79 / 38.04 | 16.64 / 19.83 | 1 / 1 |
| clocks/reused_format_microseconds | 25.97 / 28.06 | 17.22 / 19.00 | 12.50 / 12.81 | 0 / 0 |
| clocks/format_seconds | 43.19 / 44.03 | 41.42 / 43.22 | 21.76 / 34.77 | 1 / 1 |
| clocks/reused_format_seconds | 27.48 / 28.98 | 23.98 / 26.18 | 16.28 / 17.25 | 0 / 0 |
| clocks/format_hours | 48.80 / 51.98 | 39.82 / 44.41 | 23.07 / 35.18 | 1 / 1 |
| clocks/reused_format_hours | 31.53 / 33.48 | 23.08 / 23.77 | 18.21 / 18.85 | 0 / 0 |
| clocks/format_maximum_go | 54.96 / 56.74 | 49.05 / 51.45 | 32.50 / 46.67 | 1 / 1 |
| clocks/reused_format_maximum_go | 39.13 / 42.23 | 28.25 / 32.43 | 26.56 / 27.84 | 0 / 0 |
| clocks/stopwatch_paused | 1.25 / 1.27 | 1.25 / 1.35 | 6.17 / 6.44 | 0 / 0 |
| clocks/countdown_paused | 1.27 / 1.37 | 1.25 / 1.26 | 6.51 / 6.83 | 0 / 0 |
| clocks/stopwatch_foreign | 1.25 / 1.26 | 1.25 / 1.32 | 6.11 / 6.43 | 0 / 0 |
| clocks/countdown_foreign | 1.25 / 1.35 | 1.25 / 1.25 | 6.66 / 6.88 | 0 / 0 |

Reverse preparation writes digits directly into one bounded safe stack buffer. It removes integer/fraction temporary arrays and copies, preserves exact-size String allocation, checked UTF-8 and the existing zero fast path. All 269 source duration fixtures and Rust Duration::MAX remain covered.
Short 1,000-iteration runs were noisy, including unchanged zero and Go paths. All were retained, then both variants and both Go controls were rerun at 20,000 iterations (31 samples, normal 100 warmups). The table shows this longer run; raw records include both. Timings do not establish stable CPU frequency or a causal change on the unchanged zero path.
Fresh maximum shared duration changes from p50 54.97 to 49.05 ns; source after is 32.50 ns. Reused-buffer maximum changes from 39.13 to 28.25 ns versus source 26.57 ns. Fresh strings remain slower than source in all six fixtures, and both reused paths allocate zero. Clock-module performance remains pending.

Reproduce using `MATEA_PERF_ITERATIONS=20000 <probe> --clocks` on separate baseline/current release binaries and the pinned Go probe, sequentially with no compilation/tests. Retain the shorter runs and the forward context; run `python3 scripts/report_clock_format_performance.py` after collecting all named raw files. The reporter currently expects the three disposable probe paths recorded in the script.
