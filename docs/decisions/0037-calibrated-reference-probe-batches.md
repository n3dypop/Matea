# ADR 0037: Calibrate timing batches without editing timing samples

Status: accepted for reference performance tooling.

A previous Windows run produced zero timing samples with a short fixed Go
batch. Raising a single global count enough for tiny methods made expensive
render cases exceed the process timeout. Output-only records solved correctness
checking but supplied no measurement-mode portability evidence.

Calibrate each case independently after its cold call and warmup. Starting with
the requested iteration count, double up to a checked limit until the batch
reaches twenty milliseconds by default in three consecutive observations at that
count. A single scheduler pause can exceed the target even for a tiny batch;
the Windows counting fixture exposed that failure in run 39078434797. Two
confirmation durations prevent that isolated observation from choosing the
sample count. One millisecond with confirmations still failed the actual Windows
counting fixture in run 39079231800. Use a longer twenty-millisecond default and
one non-inlined timed-batch function for both calibration and retained samples,
so context-dependent loop dispatch/inlining cannot change the calibrated work.
The logs establish the short-batch failure; they do not isolate a particular
Windows hardware clock or scheduler cause. All three durations are retained; readers still accept the earlier
single-observation metadata as historical data. Record every calibration observation,
including zeros, and the actual chosen iteration count. Collect all 31 timing
samples using that count. Calibration observations are not timing samples; no
sample is dropped, clamped, retried selectively or replaced. Nonpositive timing
samples fail explicitly. Failure to reach the target or invalid bounds fails
before publishing that case. The limit also bounds arithmetic and total work.

`MATEA_PERF_MIN_SAMPLE_NS=0` explicitly retains fixed-batch operation for controlled
historical reproductions; the caller must choose a sufficient count. Positive
values select a target up to one second. Historical records without calibration
remain valid, and their original counts are not rewritten. New readers validate
the complete calibration sequence, target, bounds and final recorded count.
Output-only mode executes once and ignores timing settings.

Five Go tests cover coarse clocks, saturated growth, failure bounds, settings,
actual operation counts across all passes, output-only behavior and writer
failure. Python corruption fixtures verify the optional metadata and keep zero
samples invalid. Source CI executes actual timing/allocation mode for the 84
retained plus seven cursor cases on every OS; it checks all outputs and positive
samples but publishes only schema/count diagnostics. Shared-runner distributions
are not performance comparisons. Native module performance remains separately
reviewed.

Retain the Go checksum cohort in the repository, normalize its line endings, and
check it after module preparation. Offline validation disables both proxy and
checksum-database network access while still checking cached modules against the
retained go.sum. A missing offline dependency remains a failure.
