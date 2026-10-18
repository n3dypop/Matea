# ADR 0045: Reusable owned CIELAB terminal color gradients

Status: implementation in progress; API provisional, no parity or performance verdict.

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
