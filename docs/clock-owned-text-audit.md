# Owned clock Text construction

Status: implementation at 6ffc1eae6cadb7204c4ac45cee14579d198de8e7;
actual-library timing, local/Miri, three-OS and final assessment-input local/LLVM
qualification complete. Clock performance stays 50
and Foundation remains before F5.

ClockText::into_text returns the existing immutable renderer Text directly from
the bounded duration formatter. Stopwatch/Countdown views now use this path.
Display still handles formatting width/precision and into_string still returns
an owned String. The conversion keeps Text validation and owning copies;
it introduces no unsafe constructor, allocator, dependency or crate.
[ADR 0073](decisions/0073-owned-clock-text-without-format-adapters.md).

The original prototype compares String, existing formatting-argument Text,
private direct Text and actual pinned Bubbles Timer.View on six shared inputs.
Three orders retain 72 records/2,232 samples. A second experiment rebuilds the
actual public API and includes all prior controls in the same executable:
90 records/2,790 samples. The Go helper binary is identical between experiments.
All 162 records and 5,022 samples are independently reconstructed from original
process output and attempt journals by the [verifier](evidence/clocks-owned-text/verify.py.txt).
The six repeated inputs are not new capabilities.

| Input | Existing Text p50 range (ns) | Actual API p50 range (ns) | Source p50 range (ns) |
| --- | ---: | ---: | ---: |
| zero | 15.79–15.94 | 5.77–5.79 | 10.57–10.72 |
| nanoseconds | 24.38–24.58 | 13.99–14.08 | 15.83–15.88 |
| microseconds | 31.08–31.47 | 21.52–22.06 | 16.76–17.00 |
| seconds | 42.50–43.75 | 33.15–33.69 | 22.37–22.71 |
| hours | 32.23–32.55 | 25.90–26.07 | 21.90–22.22 |
| maximum shared Go duration | 51.82–52.38 | 40.49–40.74 | 31.77–32.13 |

All eighteen actual-library cohort/input medians improve over the existing Text
path. Both Text paths allocate zero in this measured range. The String control,
every calibration batch, cold observation and all p95 values remain in the
[raw comparison](evidence/clocks-owned-text/verification.json). Four actual API
inputs remain slower than source in every cohort. One seconds trial has p95
90.78 ns versus the old Text path's 44.66 ns; it is retained as an adverse
observation. Later seconds p95 values are lower, without establishing a cause
for that first outlier. No sample or cohort is excluded.

Each Rust process first validates all 269 fixed source strings and Duration::MAX;
all four rebuilt paths then emit identical six-case correctness records. The
maintained native clock tests now check into_text alongside Display/String for
all fixtures and maximum duration. Eight clock tests and the full 47-command
suite pass: Rust 277 plus 12 doctests, Python 146 with two conditional skips.
[Miri seed 223](evidence/clocks-owned-text/miri/context.json) passes three targeted
tests covering the formatter, themed Unicode views and Runtime reconciliation.
These are interpreted pure paths, not native terminal FFI or a soundness proof.
All 115 compile/test inputs match the committed source. The original Miri journal
omitted its producer-script hash; the retained script is hashed at verification
time only. That metadata limitation and the first verifier's failed assumption
are retained separately, without inventing an execution-time hash.

The [structural delta](evidence/clocks-owned-text/structural-delta.json) restores
the old source exactly when removing the new API, reverting its import/view
construction and removing the two added assertions. Clock state, IDs, scheduling,
legacy formatting and every prior test assertion are unchanged. Five existing
issue assessments were reopened by their source hashes before current-source CI
revalidation; the earlier successful reviews alone did not close them.

The [five original issue reviews](evidence/clocks-owned-text/issue-revalidation.json)
are now separately revalidated after current-source CI. Their old scope and
evidence hashes remain in that record. No earlier source observation, issue-body
fetch or timing run is relabeled as a new execution.

The first CI dispatch omitted reference_api and therefore skipped reference
probes. Its [negative record](evidence/clocks-owned-text/ci-dispatch-negative/context.json)
is retained and does not qualify seven required jobs. The corrected invocation
[39522044053](https://github.com/n3dypop/Matea/actions/runs/39522044053) uses both
required inputs and actually schedules all seven jobs at the same source commit.

All seven jobs succeed. The [independent artifact binding](evidence/clocks-owned-text/ci-6ffc1ea/source-bindings.json)
matches all 198 inputs to Git, verifies 47 native checks on every host and checks
uploaded log hashes. Native/instrumented terminal fixtures also pass twelve cases
per Unix host and nine in disposable Windows consoles. Raw CI bytes are retained
under the existing protected terminal-device evidence path, preserving Windows
line endings. Missing uploaded logs are explicitly listed. Pinned clock edges,
269 duration strings and sixteen output cases match the previous source checks
on all three OSes. Four portability assessments return to 75; performance stays
partial. LLVM execution counts do not prove branch coverage or memory safety.

The [final assessment-input suite](evidence/clocks-owned-text/final-native/metrics.json)
passes all 47 commands, Rust 277 plus 12 doctests, Python 146 with two conditional
skips and LLVM 95 source paths/102 instances. Three transitive hashbrown zero-hash
profile warnings are retained and diagnosed. The final gate remains 49 axes and
822 issue reports pending; Stage A continues.

Timing measures owned numeric display text creation and byte-length consumption,
excluding scene construction, rendering, scheduling and terminal IO. Short inline
storage is not a public allocation guarantee; wider Rust durations can allocate.
Rust requested allocator bytes and Go MemStats/tiny rounding differ. No retained
memory or physical RSS was measured, and no CPU affinity/frequency/ASLR control
is imposed. Active scheduling, cancellation, notification latency and complete
module performance remain pending. Stage B, F5, image engine and CUDA stay gated.
