# Help: implemented-module performance review

This qualifies the implemented Help contract. Rich per-field Lip Gloss
decorators and full reference export/feature closure remain partial. Stage A
continues; this does not start Stage B or F5.

## Executed scope

| Public operation family | Executed coverage |
| --- | --- |
| Default/new data, Clone and configuration | Default separator/style construction and drop; width/mode/style configuration, clone independence and passive event control |
| HelpItem from typed Binding, HelpMap | Borrowed enabled metadata; short/full mode dispatch through actual owned Views |
| Short text and caller-owned append | Unbounded/40/80/zero width, Unicode, 4,096 clipped candidates, controls and zero-width cluster normalization; separate reused-capacity controls |
| Full text | Ordinary/Unicode columns, 4,096 rejected rows, 1,024 visible rows, disabled wide metadata and maxima contributed by different rows |
| Structured short/full views and styles | Six owned 80x4 surface workloads consume every symbol, style, width and continuation flag; independently configured key/gap foreground/background/underline |
| Width and whole-column rejection | Fresh 102-case independent fitting replay; native/Miri tests cover all widths 0..60, early/late rejected prefixes and visible capacity growth |

There are 25 Rust controls and eight equal-output pinned Bubbles paths in each
actual-library comparison. Constructors, configuration, data preparation and
clipping are included only where named. Group-reference and binding metadata
preparation precedes rendering in both languages. Simple getters, style tokens
and representation also receive source review; this is not a separate timing
guarantee for every expression or a measurement of arbitrary terminal fonts.

## Redevelopment and retained negative evidence

1. The initial bounded-capacity candidate removes unseen work but grows the
   visible 1,024-row vector repeatedly: requests increase 3→9 and requested bytes
   72,863→137,375. Preserve all three cohorts.
2. A four-row stack prefix restores the tall column's original three requests,
   but ordinary full help rises to 418–428 ns versus 382–387 ns. Preserve that
   regression and its complete distributions.
3. A separate four-case selection experiment tests exact reservation for columns
   of at most four metadata entries. It returns ordinary full help to 380–381 ns
   against the actual preceding library's 419–422 ns. The adopted public library
   then receives a fresh complete three-order comparison.

[ADR 0074](decisions/0074-width-bounded-help-preparation.md) describes ownership,
the monotonically increasing width test and the small-column allocation tradeoff.
No new public type, dependency, cache or unsafe code is introduced. A large column
that passes five rows and fails later can still have reserved its remaining
enabled count; this does not guarantee a constant hidden-memory bound.

## Actual adopted library

Source: `63ef696`. Baseline: `71c9e60`. Both use the default release profile and
normal allocator. The old Help source compiles in a separate private crate,
differing only in its Binding import path. The candidate-selection copy equals
the adopted production body after formatting/import adjustment. The actual
pinned Bubbles/Lip Gloss source remains outside the repository; references and
manifests are hash-bound. Only equal-output cases receive a source comparison.

| Fresh plain output | Baseline p50 range (ns) | Adopted p50 range (ns) |
| --- | ---: | ---: |
| short, 40 cells | 238.96–243.42 | 240.58–242.35 |
| full, 40 cells | 381.12–386.50 | 379.35–382.51 |
| full, Unicode | 509.27–513.01 | 502.66–509.65 |
| short, 4,096 clipped candidates | 10,546–10,594 | 194–197 |
| full, 4,096 rejected rows | 189,467–192,929 | 206–208 |
| full, 1,024 visible rows | 27,094–27,503 | 26,008–26,234 |

The rejected-column case changes 4 requests/262,386 requested bytes to
3/242. The visible tall column retains 3/72,863 in every adopted cohort.
Ordinary and Unicode full outputs retain their original request counts/bytes.
All 24 adopted source/cohort medians are lower than the matched source. This is
not a whole-reference or universal maximum-speed claim. Across all 75 adopted
Rust case/cohort comparisons, 29 medians and 27 p95 observations are higher than
the old binary; these include unchanged controls and small mixed differences.
No adverse observation is discarded, causally explained without evidence or
averaged away. The identified repeated-growth and ordinary-full-help regressions
receive explicit fixes rather than a relaxed acceptance threshold.

Whole-fixture child RSS medians are 3,457,024 bytes for the baseline, 3,325,952
for the adopted path and 11,517,952 for source. The adopted maximum is 3,702,784,
above the baseline maximum of 3,473,408. These are process high-water values,
including caller setup; they are not per-operation retained memory. Rust request
counters and Go MemStats/tiny accounting remain different measurements.

The [independent verification](evidence/help-bounded/final-verification.json)
reconstructs 538 records, 16,678 positive intervals and 279 distinct RSS children
across all candidates, including the separate 16-record selection. Each ordinary
record retains all 31 samples, confirmed 20 ms calibration, first/cold observations
and allocation accounting. No compilation, test, Miri, LLVM or downloads overlap
ordinary timings. Affinity/frequency/ASLR are uncontrolled. Results describe this
native macOS arm64 host; CI timings do not establish other-host speed.

Two collection failures remain explicit. The first completed all ordinary samples
before sandbox denial of `/usr/bin/time -l`; a completed recovery journal repeats
only the RSS children. The second completed samples and RSS before incorrectly
treating its own stdout log as changing input; a separate journal verifies every
actual compile input and reconstructs all records without rerunning measurements.
The adopted collector snapshots compile inputs only and completes normally.
The initial private caller lifetime compile error is archived as a harness error,
separate from production tests. Archived callers are historical reproductions.

The final native suite initially passes 46 of 47 named commands; the PTY command's
first startup produces no bytes before its timeout. Only that command is retried
in a separate permitted launch context and passes. The original report/log and
retry report/log remain separate; consolidation verifies identical source/tool
inputs and every log hash before reusing the 46 successful commands. This does
not assign a proven cause to the original timeout or relabel its execution.

## Qualification boundaries

All four actual Help regressions pass under Miri seed 229, including safe prefix
movement/drop, early and late rejection, Unicode cells and tall visible output.
The fresh 102-case width replay exactly matches the retained independent fitting
model. The earlier candidate's seed 227 result stays separate. Three-OS/MSRV/
pinned-reference CI at the adopted source passes all seven required jobs; final
local acceptance and LLVM reports bind the subsequent assessment metadata.

The implemented-module performance assessment can reach 75 within this finite
review. The reference assessment remains 50: arbitrary rich decorators, complete
compiler/dependency/target closure and source equivalence are still pending.
Physical emulator behavior, terminal transport and framework-wide scheduling/
cross-module optimization retain their own separate obligations.
