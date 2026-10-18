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
batch operation counts and allocation guard cleanup. All eight pass Miri at seed
193 with the same measurement source hash. At e54118d all 33 native checks pass:
164 Rust tests plus two doctests, 114 Python tests, 14 snapshots and 19 Unix PTY
cases. LLVM records 62 source files and 67 compiler instances. All source inputs
and logs are retained in [the native cohort](evidence/rust-calibration/native/local-validation.json).

All ten release CLI modes complete after owned local builds, tests, Miri, LLVM
and downloads stop. They produce 282 mode records, covering 266 unique cases;
the separately invoked clock mode repeats 16 main-mode cases. Each has 31 positive
intervals, a complete three-observation calibration history at the 20 ms target,
and exact semantics/output/first-return identity with its correctness-only run.
All raw data and source/binary hashes are retained in
[the collection context](evidence/rust-calibration/confirmed/context.json).
The retained [collection procedure](evidence/rust-calibration/collection-procedure.txt)
is also hash-bound. No individual interval is retried, filtered or clamped.
This is one macOS arm64 host, without affinity/frequency control or RSS evidence.
These distributions validate the new measuring path, not an improvement over an
older measuring loop or complete module/feature coverage.

The [three-OS/MSRV/reference run](https://github.com/n3dypop/Matea/actions/runs/39106571669)
passes all seven required jobs. [Independent source/artifact binding](evidence/rust-calibration/ci-source-bindings.json)
checks all 112 program/128 execution-input hashes, 17 uploaded log hashes per
host, LLVM inputs, 14 attribute outputs and unchanged representation sizes.
Each host executes 91 confirmed Rust cases and 91 confirmed Go cases at the
20 ms target, matching each probe's own complete correctness outputs. All five
invalid Rust configurations fail before any record. These are portability/schema
checks, not cross-language speed measurements. Unix drivers pass 33 checks and
LLVM 62 files/67 instances; Windows passes 32 and 61/65. Windows PTY and physical
terminal behavior remain unverified. The b2a3558 run was superseded and cancelled
after the local fixed-one failure; it is not a passing CI result.

This changes the measuring loop and warmup work. Do not join old fixed-count data
with new calibrated data as a speed comparison. Future before/after bodies must
use the same harness revision, complete matched semantics/outputs and an explicit
policy for adaptive counts. The historical comparator continues to require equal
counts. Existing source and performance gates remain closed; no quality scores
are promoted by introducing a better measurement utility.
