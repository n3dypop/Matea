# ADR 0031: Measure reuse of the existing integer formatting dependency

Status: experiment under measurement; no public API change.

The retained six owned clock strings remain slower than the pinned Go duration strings. Caller-buffer and full owned-string costs must be kept separate, with Go controls and unchanged zero/rejected-event paths retained. Changing the result type, forcing an application allocator or omitting ownership would not be a matched optimization.

`itoa` 1.0.18 is already a production transitive dependency of locked `compact_str` 0.9.0. Its cached manifest declares MIT OR Apache-2.0 and Rust 1.68, below Matea's 1.88 MSRV. Add an explicit pinned dependency edge and use its safe `Buffer::format(u64)` for integer segments. Keep the 40-byte duration buffer, fraction formatting, final checked UTF-8, exact-size String ownership, and Display padding/error behavior. No new unsafe code enters Matea; itoa has internal unsafe and is not represented as unsafe-free. It uses an additional bounded temporary stack buffer, whose optimized frame layout is not a measured residency guarantee.

The initial experiment replaces only integer digit extraction, so any regressions remain attributable and reviewable. Run all 269 fixed Go duration fixtures, Rust Duration::MAX, targeted Miri, and matched 31-sample before/after owned/reused/ignored paths. Use longer batches and unchanged Go controls; do not discard the unchanged-path regressions or infer scheduling/full clock-module parity. Retain or revise the experiment based on the measurements.

## Experiment result and revised candidate

The isolated integer-only itoa run regressed five of the six reused-buffer medians (e.g. hours 20.34 to 26.38 ns, maximum 27.48 to 33.93 ns); one owned nanosecond case improved while the other nonzero owned cases regressed. Retain all samples, including unchanged Go and zero controls. Reject that direct dependency edge and candidate rather than claiming a win from its zero-allocation buffer.

A second private candidate generates a 200-byte decimal-pair table at compile time and prepends two digits per division for longer integer segments. One-digit segments retain their direct digit path; fractions and checked UTF-8 stay unchanged. No source is copied from itoa, and no dependency or unsafe code is added. This reduces loop/division work but adds a small read-only table; retain it only if the complete output-checked owned/reused measurements justify the tradeoff.

One earlier integer experiment was collected before CI-log downloads had finished; its isolation was not established. Its raw/context files are retained and explicitly excluded from comparisons. A separate isolated run above is the decision input.

The decimal-pair candidate did not show a sufficient overall improvement: six reused-buffer medians were unchanged or slower (hours 20.34 to 21.37 ns, maximum 27.48 to 29.05 ns), while the owned nanosecond case improved and Go controls varied by about 3%. Reject the additional table rather than treating one favorable result as a general win. The next candidate restores the original arithmetic and marks `formatted` / `as_str` inline, allowing the existing inline owned conversion to see private formatter code across the crate boundary. Measure it with the same workloads; source-level annotations alone are not performance evidence.
