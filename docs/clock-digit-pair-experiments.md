# Clock formatting: rejected arithmetic candidates and revised measurement scope

Production is unchanged at the qualified Kitty checkpoint. The clock performance
assessment remains 50 and the global gate has 48 unmet axes. Four isolated
arithmetic candidates have correct output, but none passes the fixed/varied
performance review. [ADR 0076](decisions/0076-clock-formatting-selection-and-validation-boundary.md).

The [combined verification](evidence/clocks-digit-pairs/verification-all.json)
binds 468 records and 14,508 positive calibrated intervals, all adverse cohorts,
two actual-library CPU profiles and 200 unchanged execution inputs. Three
standalone experiments each retain 72 records/2,232 intervals. A fourth, broader
experiment retains 252 records/7,812 intervals. These repeated records have
different declared consumption boundaries; they are not 468 distinct features.
No ordinary timing overlaps builds, tests, Miri, coverage, downloads or profiling.

## Why the candidates were not adopted

The first candidate writes two integer digits at once, trims fractional trailing
zeros, then restores required leading zeros. The second preserves the old short
fraction loop and handles single-digit integers and one significant tenth
directly. The third changes only the integer helper's inline attribute. A fourth
revises small-integer branches. UTF-8 validation, safe Text construction, output
units and ownership are retained throughout.

Each Rust process checks all 269 pinned source strings, Duration::MAX and 10,000
deterministically generated wide durations against the actual ClockText API.
The unchanged private formatter is a separate compilation-boundary control;
private candidate results never stand in for an adopted production API.
Private copies retain unused String helpers, so their archived build logs contain
dead-code warnings. The maintained workspace's warning-denying Clippy suite passes.

| Standalone candidate | p50 above actual API / 18 | p95 above actual API / 18 | p50 above source / 18 |
| --- | ---: | ---: | ---: |
| Pair writes and dynamic zero trimming | 9 | 10 | 12 |
| Short-fraction/single-digit branches | 6 | 7 | 12 |
| Same arithmetic, forced integer inlining | 3 | 3 | 9 |

The inlining candidate's standalone hours median is about 19 ns versus 26 ns for
the actual library and 22 ns for source, while microseconds, seconds and the
maximum shared Go duration remain slower than source. Symbols show that changing
the arithmetic also changes which helpers are outlined. That observation
motivates controlled inlining experiments; it is not a precise runtime-cost proof.

## Broader distributions prevent selection on six fixed values

The broader caller includes all six fixed controls and six 256-entry distributions:
ns, microseconds, milliseconds, seconds, hours and nonnegative signed-64-bit
nanoseconds. The ns distribution has 224 distinct values; the other five each have
256. Exact input hashes and counts are recorded. Actual pinned Bubbles Timer.View
uses preconstructed models. Rust uses existing Duration values; each timed
operation owns every formatted result and consumes a rolling length checksum.
Construction and joined output oracles precede timing. All twelve outputs and
checksums match across the five private bodies, actual library and source.

| Candidate in broader experiment | p50 above actual API / 36 | p95 above actual API / 36 | p50 above source / 36 |
| --- | ---: | ---: | ---: |
| Pair writes | 12 | 13 | 29 |
| Short-fraction branches | 26 | 21 | 30 |
| Forced integer inlining | 9 | 7 | 27 |
| Revised small-integer branches | 9 | 8 | 27 |

The following first-cohort medians are complete 256-result batch costs in ns.
They are not individual-operation latency distributions.

| Varied scale | Actual API | Forced integer candidate | Source |
| --- | ---: | ---: | ---: |
| ns | 4,152 | 4,180 | 3,898 |
| microseconds | 7,237 | 7,205 | 5,254 |
| seconds | 8,370 | 8,023 | 6,595 |
| hours | 9,735 | 9,088 | 8,545 |
| mixed u63 | 11,879 | 10,525 | 10,278 |

The microsecond batch requests zero Rust allocations versus 256 source
allocations/4,048 MemStats bytes in this cohort. Rust is nevertheless slower.
Accounting differs between the Rust System wrapper and Go MemStats; this does
not establish resident memory, and no RSS/retained-memory claim is made.

## Actual-library profiles change the next investigation

After every ordinary experiment completed, two dedicated pure Rust children
sample the actual ClockText::into_text API. The collector starts and terminates
only its own processes. Release debug information supports attribution; these
executions publish no performance timings.

| Selected collapsed top-stack symbol | Microsecond samples | Mixed samples |
| --- | ---: | ---: |
| Text::from(&str) | 1,225 | 1,071 |
| ClockText::formatted | 704 | 1,327 |
| str::from_utf8 | 604 | 307 |

The [raw profiles and bindings](evidence/clocks-digit-pairs/profile/context.json)
retain other frames and the reporting threshold. These observations are neither
exact exclusive CPU percentages nor a causal attribution of every instruction.
They show that further integer-loop changes alone do not address all sampled
work. The next investigation is a construction path that maintains valid UTF-8
and display normalization through types and ownership, with the current API kept
as the control. Removing checks or exposing an unchecked public escape is not an
accepted optimization. Public API changes require their own provisional design
and actual-library qualification before adoption.

The native 47-command/LLVM checkpoint remains current because every one of its
200 inputs is unchanged; the combined verifier checks that fact against committed
blobs. The [ordinary PR CI at 52b37e2](https://github.com/n3dypop/Matea/actions/runs/39585946691)
also succeeds; the distinct seven-job source qualification remains bound in the
Kitty audit. No completed native suite is repeated. Active scheduling, cancellation,
timeout delivery, retained memory and whole-clock-module performance remain
pending. Foundation Stage A continues; Stage B, F5, image engine and CUDA remain
gated.
