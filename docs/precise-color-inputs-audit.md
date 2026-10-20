# Explicit terminal color depth and premultiplied stops

RGBA16 style input is implemented under [ADR 0068](decisions/0068-explicit-color-depth-and-premultiplied-gradient-inputs.md).
Current-source native/Miri and all-platform CI qualification passes. The bounded
Progress performance and six changed-unit portability verdicts are 75. All 46
native checks and 92-source/99-instance LLVM collection pass again after assessment metadata updates; the 941623d
native/CI execution inputs remain archived with their original assessment values. This
increment does not advance Stage B/F5 or implement an image engine or CUDA.

## Behavioral and Rust ownership contracts

`Color::rgb16` always discards the low eight bits. An exhaustive native test checks
all 65,536 channel values, monotonicity and 256 equal-sized quantization buckets.
The existing Color/Style/Cell representation is preserved. Unknown terminal
defaults remain distinct from known black; no input magnitude guesses its depth.

`Gradient::from_premultiplied_rgba16` borrows only during construction and owns
validated stops afterward. Nonzero alpha requires each channel <= alpha; errors
identify the stop and occur before allocation. Prefix counts and single stops
retain original premultiplied projections. Blended endpoints use integer
unpremultiplication; Lab uses full normalized 16-bit channels. Zero-alpha
preserved channels use the source's documented opaque high-byte recovery.
No compositing over an unknown terminal background is inferred. The immutable
borrowed sampling iterator retains no caller input alias and allocates nothing.

The fixed-source fixture generator, raw public RGBA values and independent SGR
replay are retained in [fixture evidence](evidence/precise-color-inputs/fixture-verification.json).
Tests execute 144 gradient conditions and 576 actual Progress frames across
12 input families, prefix/interpolation/single-stop cases, width/fraction/glyph
and scaled/unscaled policies. In 548 frames source ANSI already agrees with
canonical channel conversion; 28 require the explicit correction of the pinned
source's brightness-jump heuristic. These are canonical source-color projections,
not a claim that all raw source ANSI output bytes are identical. The original
RGB8/256 indexed fixtures, invalid-input rollback and caller independence remain
tested. Single-stop huge lazy ranges and rotated grids exercise the constant path.

## Retained initial measurement

The [initial report](evidence/precise-color-inputs/performance/report.json) binds
f96bbcd and ca7214a source inputs, binaries, complete outputs and measurement
procedures. It retains 204 timing records (31 positive samples each), all
calibration batches/confirmations, separate allocation passes, and 186 distinct
child RSS observations. The lossless RSS archive has member hashes and the
reporter also supports archive-only replay. No builds/tests/Miri/LLVM/downloads
overlapped ordinary timing; all adverse values remain available.

44 legacy workloads use before/after/after/before order. Every output, first
return value, semantics string, allocation count and requested byte count agrees.
The median-of-cohort ratios range from 0.973 to 1.041. The narrow-width case
slows from 368–369 to 383–384 ns; this is retained as an adverse observation,
not described as a speedup or silently filtered out.

Four new Go/Rust/Rust/Go workloads have matched full outputs and operation scope:

| Operation | Pinned Go cohort medians | Matea cohort medians | Allocations Go / Rust |
| --- | ---: | ---: | ---: |
| Half-alpha fresh 80 colors | 23.43–23.59 µs | 4.48–4.49 µs | 83 / 2 |
| Half-alpha full 40-cell frame | 64.71–65.25 µs | 5.14–5.16 µs | 588 / 5 |
| Lost-alpha fresh 80 colors | 25.21–25.35 µs | 4.49–4.53 µs | 84 / 2 |
| Lost-alpha full 80-cell frame | 136.49–136.59 µs | 10.51 µs | 1,130 / 5 |

Fresh operations include construction, normalized output allocation and every
color's checksum. Frames include scene construction/render and checksums of all
cells/styles. No TTY transport or animation is included. Go TotalAlloc and Rust
requested System bytes differ in accounting; neither is retained heap. Whole
fixture-process RSS includes runtime/startup and is not per-operation residency.

Six additional Rust controls have independent literal oracles and no Go
constructor ratio. Initial single-stop construction plus three samples/drop is
91 ns; half/lost/opaque two-stop controls are 177/181/181 ns. Invalid alpha and
13 explicit depth conversions allocate zero times. These controls identified
unused single-stop Lab conversion and redundant opaque normalization. The
follow-up skips those operations and bounds integer products to u32 without
unsafe code.

The [follow-up report](evidence/precise-color-inputs/performance/optimized/report.json)
retains another 224 timing records and 217 child RSS observations. All 44 legacy
outputs/returns, allocations and requested bytes still match. Both measured
precise implementations preserve every output and allocation, and the four
source comparisons match full results. Single-stop construction/sample/drop
changes from 90.33–95.21 ns to 39.07–39.14 ns; it still owns one allocation.
The two-stop constructor changes are small/mixed and are not claimed as stable
speedups. Complete fresh/frame workloads remain close to the first implementation.
Current source-comparison medians are approximately 5.2–13.3 times faster in these
four operations, with no inference to other operations or machines.

The narrow legacy case's follow-up cohorts are 376.74/383.38 ns versus
375.39/375.59 ns. Another short cell-iteration cohort is slower at 710 ns versus
651–662 ns (the other current cohort is 652 ns). Retain both; do not select the
favorable cohort or claim every case improved. Larger work and allocation counts
show no material regression in this corpus. The bounded Progress performance
verdict returns to 75 with these limits, while full source styles remain pending.

## Remaining scope

The [native binding](evidence/precise-color-inputs/native/source-bindings.json)
verifies all 192 execution inputs at 941623d, all 46 acceptance logs, 92 LLVM
source paths and 99 compiler instances. The native Rust suite passes 269 tests
plus nine doctests; optimized Python passes 142 tests with two platform-dependent
skips. The [Miri binding](evidence/precise-color-inputs/miri/verification.json)
verifies 110 crate/source/fixture/manifest inputs and 26 tests across five targets
at seed 223. All 144 new gradient, 576 Progress and 65,536 quantization conditions
execute under Miri; selected legacy fixtures retain their documented subsets.
The three LLVM warnings were independently diagnosed again as missing zero-hash
transitive hashbrown DefaultHasher records; logs are retained without suppression.

[CI 39451395348](https://github.com/n3dypop/Matea/actions/runs/39451395348)
targets 941623d and all seven required jobs pass. The [independent binding](evidence/precise-color-inputs/ci-941623d/source-bindings.json)
checks all 192 execution inputs, 46 checks per OS, uploaded log hashes, ten complete
precise-color outputs per host and six literal constructor/control oracles.
Unix coverage records 92 sources/99 instances; Windows records 91/97. Six changed
units qualify for portability; no shared-runner timing is performance evidence.

Issue #1038 now has a 102-case independent width reproduction and bounded Matea
outputs; indexed-color issue #143 is reassessed after gradient source drift. The
live pending issue count is 824. The prior generated snapshot printed 826 in its
raw gate output but incorrectly copied 825 into a derived field. Its original
snapshot is preserved; the current generator derives counts and checks agreement
with raw output before publication. No original executed log is rewritten.

The ordinary host is macOS arm64 without affinity/frequency guarantees. Native
measurements do not establish universal maximum speed or another OS's timing.
CI must bind the final source before all-platform qualification. Percentage
geometry/MaxWidth/transform, hyperlink metadata, separate whitespace styling,
and complete module/declaration/issue coverage remain open. Progress reference
equivalence stays 50; this increment cannot close the entire Foundation gate.
