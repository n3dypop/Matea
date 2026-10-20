# ADR 0068: Explicit depth at color input boundaries

Status: implemented; verification in progress; public API remains provisional.

Source observations expose ambiguous 8/16-bit ANSI conversion and different
premultiplied-alpha behavior between gradient prefix and interpolation paths.
[Color/style observations](../progress-color-scope-audit.md) and
[71 alpha/type/alias observations](../evidence/progress/source-alpha-scope/context.json)
precede this decision. This is Foundation terminal styling, not image processing.

Add `Color::rgb16(red: u16, green: u16, blue: u16)` as an explicit canonical
conversion to the existing RGB8 variant. Always discard the low eight bits;
never guess input depth from its magnitude. Preserve the current enum/Style/Cell
layout and keep unknown/default terminal colors distinct from known black.

Add `Gradient::from_premultiplied_rgba16(&[[u16; 4]])`. Copy inputs into owned
stops and cache CIELAB once. Reject RGB > alpha when alpha is nonzero using an
indexed `GradientError`; accept zero-alpha preserved channels under the source's
documented lost-alpha recovery rule. No background compositing is inferred.

Retain two terminal RGB8 projections per private stop: original premultiplied
channels for prefix/single-stop paths, and normalized opaque channels for blend
endpoints. Compute Lab from full 16-bit normalized channels. Integer normalization
matches the pinned go-colorful operation order; zero-alpha recovery uses the
source's opaque RGB8 fallback. Keep the existing RGB8 constructor/fast sampling
path and measure private stop size rather than assuming ABI packing.

Nil/default input handling remains explicit: no implicit black for an unknown
terminal color and no mutation of a borrowed stop slice. Empty/invalid inputs
return errors. Callers can select an existing fill policy as their fallback.
Source nil-filter alias mutation is not carried into Rust.

Acceptance requires canonical source-RGBA projections, explicit brightness-jump
differences, prefix/single/multiple/zero-step fixtures, invalid-input rollback,
caller-array independence, precise-depth/zero-alpha cases, and actual Progress
frames. Preserve all RGB8 fixtures and distributions; measure constructor and
frame costs/allocations before keeping performance high. Run targeted Miri,
native acceptance/LLVM and all-platform CI. Percentage geometry, hyperlink and
whitespace-style coverage stays open; this decision does not close F5.

The first retained measurement (ca7214a) found a single-stop construction control
at 91 ns despite that path always returning its original channel projection.
Skip unused Lab/normalization for this case after validating it. For multiple
stops, skip integer normalization of already opaque inputs and use u32 products:
65535² fits u32 and validated channel <= alpha bounds the quotient to u16.
This keeps the same owned representation and adds no unsafe code. Preserve the
first cohort and remeasure both legacy and precise controls before qualification.
