# Retained subscription costs

Rust-only before/after measurements; no upstream or real-time parity claim.

Baseline commit: `d774b9f9a9a4a3cd4b1b53f56af34d24b3d82bdf`. [Raw data, hashes and limitations](evidence/performance/2026-10-16/subscriptions-report.json).

| Workload | p50 / p95 ns before | p50 / p95 ns after | allocations before → after | requested bytes before → after |
| --- | ---: | ---: | ---: | ---: |
| subscriptions/clock_adapter | 163.04 / 186.29 | 44.75 / 45.58 | 2 → 1 | 76 → 48 |
| subscriptions/reconcile_0 | 20.96 / 21.42 | 19.33 / 19.67 | 0 → 0 | 0 → 0 |
| subscriptions/reconcile_1 | 334.83 / 346.96 | 114.38 / 116.04 | 5 → 2 | 1230 → 144 |
| subscriptions/reconcile_12 | 2283.96 / 3297.67 | 1271.71 / 1730.67 | 40 → 13 | 5213 → 1728 |
| subscriptions/reconcile_64 | 14106.50 / 14282.50 | 7772.21 / 8120.38 | 203 → 65 | 20992 → 9216 |

Scoped identities replace repeated formatted clock IDs; validation precedes in-place retention of timer nodes. IDs move into the map key rather than being duplicated inside Timer.
Intermediate raw samples/context/source patches are retained: the zero-subscription p50 rose to 24.96 ns before an empty-configuration fast path reduced it to 19.33 ns (baseline 20.96 ns). Other short paths vary across the retained intermediate builds; do not infer a universal latency improvement.
Callback/Vec allocation remains. These per-event bytes do not measure total resident memory; larger inline keys have an unmeasured setup/storage cost.
The known-negative duplicate-ID regression lost an existing deadline before the fix. Current tests also cover zero periods, limits, namespaces, unchanged callbacks and changed revisions.
See [ADR 0021](decisions/0021-scoped-subscription-identities-and-retained-timers.md) and the full validation log for correctness. The model/effects are not rolled back on configuration errors.

Reproduce baseline by building the probe at the baseline commit, running `MATEA_PERF_ITERATIONS=1000 target/release/matea-perf-probe --subscriptions`, then repeat on the changed tree with no compilation/tests running concurrently.
Keep before context/raw files, collect after raw as subscriptions-after.jsonl, then run `python3 scripts/report_subscription_performance.py`. All 31 samples and p95 outliers are retained.
