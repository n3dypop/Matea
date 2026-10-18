# ADR 0043: Calibrate Rust micro-workload batches before comparing tiny paths

Status: proposed; no new Rust calibration implementation or measurements yet.

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
