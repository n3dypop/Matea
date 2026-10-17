# Owned scene text costs

Rust-only measurements; no upstream or whole-module parity claim.
Baseline commit: `79b538e9586aec12e82aaa54bf295da5fc939a63`. [Raw samples, contexts, intermediate patch and limits](evidence/performance/2026-10-16/views-report.json).

| Workload | before p50 / p95 ns | after p50 / p95 ns | allocations before → after | requested bytes before → after |
| --- | ---: | ---: | ---: | ---: |
| views/new_short_ascii | 39.05 / 44.80 | 16.01 / 16.48 | 1 → 0 | 9 → 0 |
| views/clone_short_ascii | 32.45 / 36.95 | 7.06 / 7.83 | 1 → 0 | 9 → 0 |
| views/new_render_short_ascii | 1113.60 / 1152.34 | 1093.68 / 1183.31 | 2 → 1 | 11529 → 11520 |
| views/clone_render_short_ascii | 1119.21 / 1169.68 | 1089.99 / 1110.99 | 2 → 1 | 11529 → 11520 |
| views/new_short_unicode | 23.99 / 24.68 | 12.18 / 12.62 | 1 → 0 | 10 → 0 |
| views/clone_short_unicode | 25.83 / 27.44 | 4.92 / 5.42 | 1 → 0 | 10 → 0 |
| views/new_render_short_unicode | 1170.52 / 1180.06 | 1148.94 / 1170.47 | 2 → 1 | 11530 → 11520 |
| views/clone_render_short_unicode | 1171.21 / 1183.72 | 1139.95 / 1222.77 | 2 → 1 | 11530 → 11520 |
| views/new_long_ascii | 28.02 / 29.40 | 86.01 / 90.90 | 1 → 1 | 87 → 87 |
| views/clone_long_ascii | 30.29 / 31.36 | 40.35 / 41.46 | 1 → 1 | 87 → 87 |
| views/new_render_long_ascii | 1734.90 / 1810.73 | 1691.91 / 2763.06 | 2 → 2 | 11607 → 11607 |
| views/clone_render_long_ascii | 1729.22 / 1744.27 | 1648.62 / 1656.60 | 2 → 2 | 11607 → 11607 |
| views/new_controls | 24.38 / 25.45 | 101.49 / 105.61 | 1 → 0 | 16 → 0 |
| views/clone_controls | 26.32 / 28.78 | 4.78 / 5.24 | 1 → 0 | 16 → 0 |
| views/new_render_controls | 1493.78 / 1505.97 | 1488.01 / 1498.72 | 4 → 1 | 11584 → 11520 |
| views/clone_render_controls | 1501.05 / 1515.73 | 1392.22 / 1403.52 | 4 → 1 | 11584 → 11520 |
| views/formatted_counter | 37.73 / 41.49 | 30.54 / 31.68 | 1 → 0 | 14 → 0 |
| views/formatted_counter_render | 1132.71 / 1146.46 | 1120.32 / 1136.06 | 2 → 1 | 11534 → 11520 |
| views/short_panel | 102.80 / 108.95 | 63.29 / 66.36 | 4 → 2 | 149 → 140 |
| views/short_panel_render | 12294.62 / 12462.89 | 12253.74 / 12442.40 | 5 → 3 | 17429 → 17420 |

Immutable Text normalizes controls once, stores small values inline and accepts format_args! without an intermediate String. Its typed drawing path skips repeated normalization of prepared leaves. Public Canvas::text still normalizes arbitrary strings.
This moves work: long-ASCII construction rises from p50 28.02 to 86.01 ns and control construction from 24.38 to 101.49 ns. Long clone also regresses in this run. Complete build/render and clone/render columns are separate cases; small timing differences and p95 regressions are retained. Do not infer universal CPU wins from allocation savings.
The first normalized stage's long constructor measured 109.75 ns. A safe UTF-8 byte scan reduces it in this run; every Unicode scalar is checked against the standard control policy. Intermediate source hashes, patch and samples remain available.
All twenty rendered-output fixtures match. Correctness tests additionally compare full cells/styles across themes/focus/size, immediate owned formatting and failure semantics. The independent-widget example uses only facade contracts.
Constructor bounds are provisionally Into<Text>; custom Into<String>-only types need explicit conversion or Into<Text>. See [ADR 0022](decisions/0022-owned-normalized-scene-text.md) and [Text API](text-api.md).

Reproduce baseline/current release probes using `MATEA_PERF_ITERATIONS=20000 target/release/matea-perf-probe --views` with no compilation/tests running concurrently. Retain before context/raw and the intermediate data, then run `python3 scripts/report_view_performance.py`.
