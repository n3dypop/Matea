# Strict measurement record validation

Five failing assertions at 29ffeea are retained in
[the negative fixtures and original failure log](evidence/measurement-integrity/before-context.json).
The modern comparison reader accepted duplicate JSON fields, forged correctness
records, calibration that never reached its target and negative cold duration.
The historical reader also accepted negative cold duration. These are validator
defects; the retained production samples are not known to contain those values.

Both readers now share duplicate/nonfinite JSON rejection and measurement
validation. A negative, boolean or nonfinite cold duration is rejected; legitimate
zero and historical absence remain valid. Modern records still require exactly
31 samples, integer counters and a first return value. Historical fractional Go
counters and missing first returns remain explicit. The fixed-count comparator
is unchanged. Nested duplicate fields, overflowing float literals, malformed
calibration, correctness/timing confusion and direct nonfinite Python values are
covered by regression checks. Decoder reuse preserves UTF-8/16/32 input handling
and does not retain state after a rejected document.

[All five formerly failing assertions now reject](evidence/measurement-integrity/after-checks.json).
[Five historical reporter CLIs](evidence/measurement-integrity/historical-cli-checks.json)
pass with `--check`, without replacing published reports. Normal and optimized
Python execution use explicit checks; no correctness assertion depends on `assert`.

| Same 92 valid records / complete 46-case report | p50, two balanced runs | p95, two runs |
| --- | --- | --- |
| Original reader, first experiment | 1.320 / 1.310 ms | 1.373 / 1.391 ms |
| First shared validator, f4427a1 | 2.082 / 2.072 ms | 2.205 / 2.148 ms |
| Original reader, final experiment | 1.318 / 1.308 ms | 1.471 / 1.429 ms |
| Optimized shared validator, 3f69cc8 | 1.601 / 1.592 ms | 1.736 / 1.727 ms |

The final implementation reduces Python call/decoder overhead while retaining
strict checks. It remains slower than the defective original. The new checks
perform additional work; this is not a matched-validation speed claim. All
before/after/after/before runs retain 31 positive intervals, exact serialized
output identity and separate Python heap passes. Peak traced bytes are 288,641
in the final path versus 288,649 in its baseline; these are not native RSS or
system allocation counts. There is no file/Git/process I/O in the timed body.
No owned build, tests, Miri, coverage or download overlapped timing. One macOS
arm64 host, clock/frequency noise and the valid finite corpus limit inference.

[Initial raw/source context](evidence/measurement-integrity/reader-benchmark/context.json),
[final raw/source context](evidence/measurement-integrity/reader-optimized/context.json).
Historical functions retain their own globals and pinned parser dependency;
tests also verify the current module is not replaced by the historical loader.

At 3f69cc8, all 33 native checks pass: 156 Rust tests plus two doctests, 113 Python
tests, 14 snapshots and 19 Unix PTY cases. LLVM collection records 62 source
files and 67 compiler instances. The
[three-OS/MSRV/reference CI](https://github.com/n3dypop/Matea/actions/runs/39105769194)
is pending. Scores remain 49 axes below 75 across 112 programs and 41 units.
The containing reporting unit still needs complete workload and portability
evidence. This change does not advance Stage B or F5.
