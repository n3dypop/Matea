# ADR 0045: Reusable owned CIELAB terminal color gradients

Status: implemented and verified at 1202535 on Linux, Windows and macOS;
API provisional, full reference scope remains partial.

Pinned Lip Gloss blending.go exposes one- and two-dimensional CIELAB gradients.
Progress depends on the one-dimensional path for scaled/half-block fills. Its
segmented distribution, small-count prefix shortcut, dimension normalization and
rotation sampling are observable behavior. go-colorful v1.4.1 supplies the sRGB,
XYZ and Lab equations, white point and matrices; retain its MIT notice and source
hashes when using an altered Rust implementation. This is terminal style color
composition within Foundation, with no image processing/decoding/CUDA work.

Introduce Gradient in matea-style, owning immutable RGB8 stops with cached Lab
coordinates. Construction validates nonempty input and reserves memory fallibly.
One-dimensional iteration and lookup borrow that configuration, allocate no output
array, and preserve exact-size/double-ended/fused iterator contracts. Grid shape
is bounded by u16 terminal dimensions, promotes zero dimensions to one, checks
area arithmetic and rejects nonfinite angles. Each grid owns a fallibly reserved
diagonal palette and cached trigonometry, independent of the source Gradient's
lifetime. Borrowed grid traversals allocate no output array. Recomputing Lab->RGB
at each cell would discard the reference's palette reuse; retain that reuse
before making any performance claim.

Preserve the reference segment distribution and prefix shortcut for explicit
RGB8 stop fixtures. Return Matea Color::Rgb through the existing fallback and
style/render boundaries. Quantize via the reference RGBA16-to-RGB8 convention;
do not silently substitute RGB255 rounding. Stops are owned, and no caller slice
is mutated or returned as an alias. Cached Lab coordinates remove repeated
conversion of the same stops inside every interpolation operation.

This initial contract is explicitly bounded to RGB8 input/output. Arbitrary
color.Color implementations, fractional/high-depth or alpha composition are not
claimed equivalent. Record them as remaining reference scope. Public types and
methods remain provisional until the broader API review; no crate name is added.

Require pinned Go output fixtures, empty/single/multiple stops, small/large counts,
segment boundaries, zero/grid dimensions, nonfinite/negative/wrapped angles,
mixed-ended iterator exhaustion and near-usize-limit indexed access. Exercise
the new styles in a runnable theme/fallback/resize example and normal/native/Miri
checks. Fresh configuration plus materialization and reusable iteration are
different workloads; compare matched complete outputs and retain both costs,
allocation counters, distributions and adverse results. No score promotion or
module parity follows from implementing this bounded feature.

The fresh cross-language workload configures three RGB8 stops, materializes the
complete RGB8 result and consumes every channel in a fixed FNV64 checksum.
Formatting and terminal I/O are outside timing. Both probes use a 20 ms target
with three confirming observations and retain 31 positive per-operation values.
Adaptive iteration counts may differ for this new cross-language comparison;
record every actual count and observation, and require exact semantics, all
output colors and first-return identity. This does not relax the historical
fixed-count before/after comparator. Keep reference/Matea/Matea/reference order
and retain both pairs, including adverse tails. The six Rust streaming controls
reuse configuration and omit the materialized output; never give them an
upstream speed ratio against the fresh workload.

All 548 RGB8 fixture cases match natively; the three ownership/iterator/fixture
tests pass Miri with 19 selected reference cases. The runnable example passes
snapshot and Unix PTY focus/rotation/resize/cleanup checks. Local acceptance has
33 passing checks, 167 Rust tests plus two doctests, 115 Python tests, 15 snapshots
and 20 Unix PTY cases. LLVM records 66 source files and 71 compiler instances.
The input snapshot now binds the known Rust CSV/TSV fixtures and dashboard golden
as well as program/build/check manifests (138 inputs, 116 programs).

Both balanced pairs match complete outputs and first returns. Fresh medians are
12–34% of the reference for the six measured RGB8 workloads, and the six distinct
Rust reuse controls request no allocations. These results do not establish
constructor/prefix/error/high-depth coverage, full styled rendering performance
or module parity. Rust requested System bytes and Go TotalAlloc bytes have
different accounting/rounding; retain that distinction. See
[raw distributions and limits](../gradient-audit.md).

All seven required jobs pass in CI 39109108150. Independently checked artifacts
bind 116 programs/138 known inputs, 17 uploaded logs per host, all 548 reference
fixtures, six matched fresh gradient outputs and all 12/6 Rust/Go calibrated
gradient paths. Unix drivers pass 33 checks/LLVM 66 files/71 compiler instances;
Windows passes 32/65/69. Existing 91-case Rust/Go cohorts also pass. Native
Windows console and physical emulators remain unverified. Scores remain 49 axes
below 75; these finite RGB8 observations do not close the module or F5 gate.
