# Owned panel token and allocation optimization

The attribute slice exposed a short-panel allocation regression: retaining a
complete Theme requests 160 bytes, versus the older 140-byte theme representation.
The private panel only consumes surface, border, accent, padding and border kind.
ADR 0042 retains those tokens and the child View in one boxed PanelContent, with
an inline title. View::panel(title, &theme) remains an owned snapshot. There is
no global theme cache, Arc, unsafe layout or public lifetime/API change.

The token-only intermediate (db865c6) keeps two allocations and requests 102
bytes. Its construction timing is mixed. The final 29ffeea groups child and
tokens in one allocation. On the measured native host, Style is 14 bytes, Cell
40, View 48 and Theme 104. These are observed compiler layouts, not ABI promises.
The regression test enforces the old no-growth View budget of at most 56 bytes;
the first draft incorrectly required equality and failed on the smaller 48-byte
result. That log is retained and is not counted as a production defect.

| Existing fixture | Full-theme baseline | Single payload |
| --- | --- | --- |
| Construct/drop short panel | 2 allocations, 160 requested bytes; 62.92 ns p50 | 1 allocation, 96 bytes; 42.29 ns |
| Construct/render/drop panel | 3 allocations, 17,440 bytes; 12.296 µs p50 | 2 allocations, 17,376 bytes; 12.283 µs |

[All 20 exact-output controls and source hashes](evidence/optimization/panel/single-allocation/report.json)
retain every percentile and counter. The first final full-render p95 increases
from 12.490 to 13.519 µs. In the six-run before/intermediate/after/after/intermediate/before
repeat, construction medians are 61.77/70.27 ns for the baseline, 61.83/61.85 ns
for token-only and 42.31/42.29 ns for the single payload. Complete-render medians
are mixed and do not establish a pipeline speed improvement.
[All repeats](evidence/optimization/panel/single-allocation/repeats.json) remain
unfiltered, including outlying constructor tails and unaffected-case regressions.

Baseline/intermediate records are reused from their committed measurements;
they were not silently regenerated. All timings run after owned local builds,
tests, Miri, coverage and downloads finish. There are 31 retained samples per
case, with requested System allocation counting in a separate execution. This is
one macOS arm64 host without affinity/frequency control or resident-memory data.
This is a component optimization in Stage A, not the later framework-wide Stage B.

The finite behavior test covers all three border kinds, padding 0/1, literal rows,
border/title/surface styles (including extended attributes), CJK continuation
cells, source Theme mutation/scope exit, cloned scenes and column natural height.
The same test passes targeted Miri at the token-only and single-payload sources.
Final color-profile Miri checks the retained Default/RGB underline fixture against
the original production adaptation code. Native acceptance at 29ffeea passes all
33 checks, 156 Rust tests plus two doctests, 105 Python tests, 14 snapshots and 19
Unix PTY cases; LLVM records 62 source files and 67 separate compiler instances.
All seven required jobs pass at 29ffeea in the [three-OS/MSRV/reference run](https://github.com/n3dypop/Matea/actions/runs/39093118277).
[Independent artifact/source binding](evidence/optimization/panel/single-allocation/ci-source-bindings.json)
checks all 111 program/127 execution-input hashes, 17 uploaded log hashes per
host and all 14 extended-attribute outputs. Every host observes Style 14, Cell 40,
View 48 and Theme 104 bytes. Unix drivers pass 33 checks/LLVM 62 files and 67
instances; Windows passes 32 checks/61 files and 65 instances. These are native
execution observations, not physical emulator or native Windows console claims.

A separate default-underline guard experiment was not adopted. Its first
144-case run regressed style pairs, while subsequent warm repeats only slightly
improved indexed-color pairs and mixed no-color/truecolor results. Production
Style::adapt is restored; the expanded boundary fixture remains.
[Decision and limits](evidence/optimization/underline-default/decision.json),
[all 144 cases](evidence/optimization/underline-default/joined/report.json).
ADR 0043 proposes a separately audited Rust batch-calibration utility; it has
not been implemented and does not retroactively calibrate these records.

Whole-reference capabilities, performance scope, 49 low assessments and the
Foundation/image-engine/CUDA gates remain unchanged. These changes do not
establish every-module parity or a universal performance maximum.
