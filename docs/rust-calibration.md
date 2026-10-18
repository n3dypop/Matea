# Bounded, confirmed Rust measurement batches

The measurement-only helper shared by the Rust probe and two custom benchmarks
now calibrates batches with a 20 ms default target. Three independent observations
at one count must all reach the target. Counts double up to 2^26; failures retain
their observation history. Calibration and the 31 retained intervals use the same
non-inlined batch loop. A zero retained interval fails without retry or clamping.

`MATEA_PERF_MIN_SAMPLE_NS=0` explicitly selects fixed counts. LLVM collection uses
this mode because instrumented timings are execution observations. Native CI
requires the actual confirmed path at 20,000,000 ns. The maximum target is one
second. CLI starting iterations must be 1..=6,710,886 so existing at-most-tenfold
caller scaling stays bounded. Malformed, non-Unicode, zero or out-of-range counts
are errors rather than silent defaults. This is private tooling, not a framework
public API change.

The first LLVM run with a fixed count of one fails on a zero coarse-clock interval
at style/adapt_default_ansi256; the partial output and failed journal are retained.
The driver now uses a uniform fixed count of 1,000 across probe modes. It does not
retry selected intervals or publish instrumented distributions as performance
evidence.

An existing 29ffeea release binary accepted zero counts and emitted NaN while
returning success. [The original raw output, exact binary and source context](evidence/rust-calibration/before-context.json)
remain retained as an invalid-configuration regression, never a performance
sample. New local CLI checks reject five invalid configurations before output.
Correctness-only mode still emits 14 valid attribute records even with invalid
timing settings; it does not evaluate those settings or run timing/heap passes.

Allocation counting remains a separate single operation. A guard turns tracking
off on unwind and rejects nested passes. These process-global requested System
counters are not RSS and are not valid for concurrent independent measurement
sessions. The forwarding allocator is unchanged, measurement-only unsafe code;
the framework's safe-code policy is unaffected.

Eight deterministic Rust tests cover bounds, three confirmations after a single
scheduling pause, all coarse-clock observations at the cap, zero-work rejection,
overflow-safe growth, all 31 intervals, failure without retry, exact stateful
batch operation counts and allocation guard cleanup. Full native/LLVM/Miri and
three-OS verification are pending. Calibrated distributions are not yet collected.

This changes the measuring loop and warmup work. Do not join old fixed-count data
with new calibrated data as a speed comparison. Future before/after bodies must
use the same harness revision, complete matched semantics/outputs and an explicit
policy for adaptive counts. The historical comparator continues to require equal
counts. Existing source and performance gates remain closed; no quality scores
are promoted by introducing a better measurement utility.
