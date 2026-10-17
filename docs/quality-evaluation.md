# Implementation quality evaluation

This is a reproducible review and test plan for **all implemented programs**, not
an assertion that Foundation or the referenced public APIs are complete.
[The explicit inventory](quality-plan.json) maps every Rust, Go, Python and CI
program file to a review unit. New unlisted files fail the inventory check.
[The generated report](evidence/quality/report.json) records each file's source
hash, assessment and available native execution coverage. No average hides a
failing dimension; `python scripts/check_quality.py --gate` fails until every
applicable assessment is at least 75.

## Scoring contract

Scores are ordinal evidence levels, **not percentages of correctness**:

| Score | Required interpretation |
| --- | --- |
| 0 | No executed evidence for this dimension/workload. |
| 25 | Known defect or review only; behavior/performance is not validated. |
| 50 | Partial evidence, missing behavior/measurements, or unresolved regressions. |
| 75 | High **within the explicitly bounded reviewed/tested scope**; independent fixtures and executed checks support it. |
| 100 | Exhaustive evidence for an explicitly finite domain; never universal safety or maximum speed. |
| N/A | The dimension has no counterpart (e.g. Rust ownership for a Python utility); a reason is mandatory. Unmeasured performance is 0, never N/A. |

The initial assessments are conservative reviewer judgments, with limitations in
each row. Passing a native test does not establish reference parity. Passing an
inventory check does not establish a passing quality gate. Tests/configuration
have no production latency acceptance criterion, so performance is N/A there;
auditors, report programs and examples still have unmeasured work and receive 0.

## Evaluation and test items

| Dimension | Review questions | Executed checks / required follow-up |
| --- | --- | --- |
| Rust design | Are state and resources owned once? Can invalid states be represented? Are borrows/lifetimes simple, cancellation and errors explicit, and extension contracts independent? | All-target Clippy with warnings denied, documentation, compile-fail immutability, standalone widget; targeted runtime identity/deadline tests. Public API/registry review remains open. |
| Memory safety | Is unsafe isolated and justified? Are arithmetic, indexing, UTF-8 slices, aliasing, mutation, cleanup and thread ownership valid? | Safe-code crate policies, bounded paint/layout models, exhaustive Unicode scalar policy, targeted Miri. The Unix sigaction boundary and performance-only System allocator wrapper are explicitly unsafe; transitive dependencies are not asserted unsafe-free. |
| Behavior | Are boundaries, empty input, control characters, wide glyph overlap, resize, stale messages, failures/panics and recovery correct? | Debug/all-target/release tests, snapshots, independent terminal trace interpreters, 17 Unix PTY cases, graphics protocol round trips, before/fix regression logs. These do not validate every physical emulator. |
| Reference equivalence | Are semantics/outputs equal at the pinned SHAs? Are deliberate improvements documented and unavailable features visible? | Finite Harmonica/paginator/key groups, clock/spinner fixtures and matched performance output checks. Revised whole-reference coverage remains incomplete; see the separate declaration/compiler ledger. |
| Performance | Are constructor and complete pipeline costs both measured? Are p50/p95, allocations, output bytes and regressions retained? | 84 reference-matched finite cases, 5 subscription and 20 scene Rust-only cases, 12 new Viewport before/after cases; long clock repeats overlap the 84. No instrumented timing is performance evidence. Missing resident memory, scheduling/application latency and module workloads remain pending. |
| Portability | Does this exact implementation run on Linux, Windows, macOS and MSRV? Are terminal/FFI limits explicit? | Foundation CI plus MSRV. Native host coverage/Miri is additional evidence, not another OS. Windows PTY and physical emulator behavior remain unverified. |

## Redevelopment performed

1. **Viewport behavior: 25 → 75 for the tested slice.** The first three added tests
   failed before the fix: trailing blank lines disappeared, CRLF became a visible
   replacement character, and a grown viewport exposed an obsolete offset before
   navigation. [The known-negative log](evidence/viewport-before.log) is retained.
   Immutable content plus UTF-8 line offsets preserves trailing lines, normalizes
   CRLF and clamps the observed position before navigation. The independent line
   model covers zero heights, controls, Unicode and navigation. A fourth test
   prevents ignored input from committing a resize position change; its
   [failing intermediate case](evidence/quality/viewport-ignored-before.log) is retained. The containing
   basic-widget module remains partial; this does not award List/Table/Form parity.
2. **Viewport allocation/performance: 50 → 75 for 12 finite workloads.** ASCII
   24-row View generation changes from 26 allocations / 1,559 requested bytes to
   1 / 503; p50 1,096.21 → 364.90 ns. Complete build/render changes from 27 to 2
   allocations and p50 11,916.92 → 11,058.44 ns. The one-row ASCII complete
   workload has a p95 regression (759.71 → 853.68 ns). All samples, output
   equivalence and workload exclusions remain in [the first report](evidence/quality/viewport-first-report.json); the final rerun is in
   [the current report](evidence/quality/viewport-report.json).
   This excludes initial parsing and resident storage, so the whole Viewport
   performance dimension remains 50 until those and matched upstream work exist.
3. **Source auditor conditions: 25 → 75 for the new nested-cfg fixture.** Rust
   field, enum variant/payload, impl/trait member, const/static/type/reexport
   conditions were omitted. [The before failure](evidence/quality/auditors-before.log)
   reproduces the lost `unix` constraint. Regenerated inventories correct 80 Crossterm and
   42 Ratatui condition records while retaining all declaration IDs/counts. Child conditions now combine with the
   parent conditions without evaluating them. Go fixtures also verify build tags,
   embedded types, exports and benchmark filtering. The complete compiler-export
   audit and scanner failure/scale cases remain separate work.

4. **Evidence validation: 25 → 75 for optimization-mode regression.** Python
   `-O` removed `assert` statements, including a complete replay call, allowing an
   invalid trace to print “exact states”. The [before failure](evidence/quality/optimized-validation-before.log)
   is retained. Replace 138 removable checks in 14 production scripts with an
   explicit `require` function; lazy messages preserve failure-only evaluation.
   Tests reject invalid traces in normal/optimized modes and prevent new removable
   checks in production scripts. All 16 Python regression cases run in both modes.

## Reproduction and limits

Run `python scripts/verify_foundation.py --offline` for the native test, lint,
release, documentation, example, Python syntax, auditor/oracle and inventory suite.
Run `python scripts/measure_rust_coverage.py` after installing matching official
`llvm-tools-preview` to collect instrumented native tests, example/PTY/CLI runs.
Its line/function totals include embedded `cfg(test)` code; they are not branch
coverage or production-only percentages. Compiler/profile raw files stay in
ignored `artifacts/quality-coverage`. Failed syscalls, process creation and rare
OS error branches still need fault-injection evidence.

The targeted Miri commands were `MIRIFLAGS=-Zmiri-seed=17 cargo +nightly
miri test --locked --offline -p matea-render --test invariants` and
`MIRIFLAGS=-Zmiri-seed=29 cargo +nightly miri test --locked --offline -p matea-core
--test subscription_atomicity --test subscription_identity`. The paint model uses
32 steps under Miri and 4,096 natively. Four additional Text ownership/formatting/
clipping tests ran with seed 41, excluding the million-scalar native test. Nine
targeted Miri tests passed in total; no full-workspace Miri claim is made.
[Miri's own documentation](https://github.com/rust-lang/miri#readme) explains why
a successful execution cannot prove soundness or explore all thread schedules.

Do not loosen thresholds or drop adverse cases to make the report green.
Address the generated below-threshold queue with independent regression tests
and fresh measurements, then update only assessments supported by that evidence.
The initial four-job CI succeeded but skipped one Go fixture on macOS. The
[initial excerpt](evidence/quality/initial-source-auditor-ci-excerpt.log) preserves
that gap. Pin official setup-go v6 to commit
`924ae3a1cded613372ab5595356fb5720e22ba16`, install Go 1.26.0 and make a missing Go
CLI fail the fixture. Require fresh CI with all two source-auditor cases and all
16 optimized Python cases executed on every OS. Windows PTY and the optional
reference recollection remain separate exclusions.

F5 and the image-engine/CUDA gate remain closed.

Final implementation `9ce5564ad525d047594c5e1debecabd7c3bb4e37` passed [Linux, Windows, macOS and MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39043211231). The generated report retains 56 below-threshold assessments: performance 28, reference equivalence 18, behavior 5 and external-tool portability 5. These are incomplete evidence/behavior scopes, not 56 confirmed defects. The quality gate still fails.
