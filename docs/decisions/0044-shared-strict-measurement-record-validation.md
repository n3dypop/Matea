# ADR 0044: Share strict measurement validation without rewriting history

Status: implemented and locally verified at 3f69cc8; three-OS CI pending.

Five failing assertions at source 29ffeea show the modern optimization reader
accepts duplicate JSON fields, correctness records with forged timing fields,
unreached calibration and a negative cold counter. The general historical reader
also accepts the negative cold counter. Valid current records are not known to
contain these defects, but accepting malformed inputs undermines report verdicts.

Use the existing performance_records validator for both readers, including typed
record separation and calibration history. Share duplicate/nonfinite JSON parsing
with measurement context loading. The modern optimization reader retains its
stricter contract: exactly 31 samples, integer counters and a first return value.
The historical reader retains missing first-return fields, fractional Go average
allocation counters and all available samples. Do not fabricate, round or drop
those differences to make records compare.

Cold timing remains optional for history; when present it must be finite and
nonnegative, including legitimate zero coarse-clock observations. Correctness
records never become timing records by adding fields. Reject invalid data before
publishing any report. Do not relax the fixed-count comparator for future adaptive
Rust calibration without the separate contract in ADR 0043.

Retain the negative fixtures/failing log, add independent normal/optimized-mode
tests, run retained valid reports and preserve adverse data. Score promotion
requires evidence for the containing complete tooling unit, not this fix alone.

The first shared-validator implementation increased the 46-case report median
from about 1.31 to 2.08 ms. Retain that result. Inline the per-sample validation,
reuse strict decoder configuration, and remove redundant modern-reader checks
already guaranteed by the common validator. The final balanced run measures
1.59–1.60 ms, versus its 1.31–1.32 ms baseline. Additional validation is intentional
work; neither equal validation semantics nor a speed win over the defective
baseline is claimed. Historical comparisons freeze both the reader and its
repository-owned parser dependency without replacing process-global modules.
See [all results and limitations](../measurement-integrity.md).
