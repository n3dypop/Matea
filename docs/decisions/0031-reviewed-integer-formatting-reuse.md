# ADR 0031: Measure reuse of the existing integer formatting dependency

Status: experiment under measurement; no public API change.

The retained six owned clock strings remain slower than the pinned Go duration strings. Caller-buffer and full owned-string costs must be kept separate, with Go controls and unchanged zero/rejected-event paths retained. Changing the result type, forcing an application allocator or omitting ownership would not be a matched optimization.

`itoa` 1.0.18 is already a production transitive dependency of locked `compact_str` 0.9.0. Its cached manifest declares MIT OR Apache-2.0 and Rust 1.68, below Matea's 1.88 MSRV. Add an explicit pinned dependency edge and use its safe `Buffer::format(u64)` for integer segments. Keep the 40-byte duration buffer, fraction formatting, final checked UTF-8, exact-size String ownership, and Display padding/error behavior. No new unsafe code enters Matea; itoa has internal unsafe and is not represented as unsafe-free. It uses an additional bounded temporary stack buffer, whose optimized frame layout is not a measured residency guarantee.

The initial experiment replaces only integer digit extraction, so any regressions remain attributable and reviewable. Run all 269 fixed Go duration fixtures, Rust Duration::MAX, targeted Miri, and matched 31-sample before/after owned/reused/ignored paths. Use longer batches and unchanged Go controls; do not discard the unchanged-path regressions or infer scheduling/full clock-module parity. Retain or revise the experiment based on the measurements.
