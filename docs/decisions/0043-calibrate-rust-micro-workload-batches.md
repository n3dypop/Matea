# ADR 0043: Calibrate Rust micro-workload batches before comparing tiny paths

Status: implemented and verified at e54118d on Linux, Windows and macOS.

The shared Rust probe currently runs caller-chosen fixed batches, warms 100
operations, retains 31 per-operation averages and counts allocations separately.
Those records remain valid observations of their stated batches, not proof of
stable nanosecond behavior. The default-underline guard experiment (ffce754)
produced an initial ANSI16 pair median of 41.98 ns versus 17.10 ns baseline, then
balanced medians of 16.79/16.79 versus 54.23/17.29 ns. No owned compilation,
tests, Miri, coverage or downloads overlapped these timing passes. OS scheduling,
clock/frequency state and short sample duration still limit the inference. The
guard was not adopted and all samples remain retained.

The pinned Go probe already uses one shared non-inlined batch loop for calibration
and retained samples, three confirming observations, a 20 ms default minimum and
an explicit fixed-batch mode. Its 91 timing/schema cases pass on all three OSes;
this does not validate a yet-unimplemented Rust counterpart.

Propose the same measurement contract for the Rust utility: a typed bounded
configuration, a shared batch operation, a minimum duration with confirming
observations and a finite iteration bound. Reject invalid/nonpositive samples;
never clamp, drop or retry selected retained samples until they look favorable.
Keep the correctness-only output mode separate, with no timing/calibration or
allocation work. Keep allocation counting outside timing, with cleanup on failure.
Instrumented LLVM paths can request fixed batches explicitly because their values
are execution coverage, not performance evidence.

Record every calibration observation, actual iterations and the floor/bound
policy. Before/after bodies must use the same harness version. Adaptive counts
may differ only under an explicit audited comparison contract; do not silently
relax the historical strict fixed-count comparator. Check stateful/cyclic
operations, output identity, first-return meaning, allocation isolation, overflow,
coarse clocks, unsupported floors and failure behavior. Use deterministic fake
clocks/counters for boundary tests and real positive-sample runs on Linux,
Windows and macOS. Preserve fixed historical data and calibration failures.

Increasing batches is not a replacement for matched operations or controlled
hosts. It establishes neither a universal optimum nor all-feature/module parity,
and does not advance Stage B or F5. Implement and verify this measurement utility
after the in-progress panel slice is recorded; do not reinterpret its older
observations as calibrated measurements.

The shared Rust utility now defaults to 20 ms confirmed by three observations,
with a 2^26 iteration cap and a 0..=1 second target. An explicit target of zero
selects fixed batches for execution coverage; existing historical measurements
are unchanged. Starting CLI counts are bounded before the callers' at-most-tenfold
scaling. Invalid configuration fails before a workload prints a timing record;
correctness-only mode bypasses timing configuration, clocks and allocation
tracking. The allocator pass uses a guard that disables tracking on unwind and
rejects nested passes. Calibration failures preserve all attempted observations,
and a zero retained interval fails at its original index without retrying it.

The 29ffeea binary accepts zero iterations and prints NaN with successful exit;
the [reproduction](../evidence/rust-calibration/before-context.json) is retained.
This is a confirmed utility defect, distinct from the earlier host-noise limits.
CI will require the exact calibration target for the current style/input/protocol
records and execute all 91 existing matched Rust/Go timing-mode cases separately.
Shared-runner values are schema/output checks, not comparative speed evidence.

At e54118d, all 33 native checks, 164 Rust tests plus two doctests, 114 Python
tests and LLVM collection pass. Eight calibration/guard tests pass Miri at the
unchanged measurement-source hash from b2a3558. The first fixed-one LLVM attempt
fails on a zero interval and is retained; a uniform fixed count of 1,000 completes
62 source files/67 compiler instances. All ten release CLI modes then produce
282 mode records (266 unique cases), each with 31 positive samples, matching
correctness-only output/first return and confirmed target history. Sources and
binary are unchanged throughout collection, and all owned local builds/tests/
Miri/coverage/downloads finish before timing. This validates the measuring path;
it is not a speed comparison against old fixed batches. [Evidence and limits](../rust-calibration.md).

All seven required jobs pass in CI 39106571669. Independently retained artifacts
bind all 112 programs/128 execution inputs and 17 uploaded logs per host to the
commit. All three hosts execute 91 confirmed Rust and 91 confirmed Go cases,
matching their own complete outputs, and reject five invalid Rust configurations.
LLVM completes with the explicit fixed mode on each host. Physical terminals and
Windows PTY remain separate unverified conditions.
