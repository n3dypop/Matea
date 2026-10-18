# ADR 0046: Generation-safe themed progress

Status: implementation in progress; provisional API, not a module parity verdict.

Implement Bubbles progress user functions using matea-widgets, the independent
matea-motion spring and the existing typed clock/subscription adapter. The live
Progress owns one non-cloneable clock identity, target and visible Motion. Cache
the 60 Hz spring matrix; accept one fixed step per later valid tick, matching the
reference scheduling model rather than performing unbounded catch-up work.
Retargeting invalidates earlier scheduling generations. Duplicate, foreign,
stale and backward observations cannot change visible state. No per-frame task,
unsafe code, terminal escape sequence or runtime-internal dependency is added.

Expose target and motion separately: upstream Percent actually returns the
target despite its visible-percentage documentation. Reject nonfinite values
before mutation, clamp finite percentages, require positive spring frequency
and allow zero damping as explicitly continuous oscillation. Settle only when
both distance <0.001 and absolute velocity <0.01, then snap exactly to target
and remove the schedule. This fixes the reproduced signed-velocity rest bug.
Reduced-motion rendering displays the target and suppresses subscriptions;
explicit reduced-motion control also snaps/stops state. Rendering never mutates
animation or consults global configuration.

Use one fill enum for theme, solid, owned RGB8 gradient or callback, avoiding
the reference's hidden callback precedence after changing gradient options.
Gradient scaling and half-block foreground/background samples retain reference
distribution. Color callbacks receive the clamped displayed fraction and cell
position; callback/configuration ownership is independent of live identity.
Styles default to semantic Theme tokens. Arbitrary high-depth/alpha Go color
implementations remain outside the current RGB8 gradient contract.

Validate fill glyphs as printable one-cell characters. Width is a terminal u16.
Percentage formatting uses an optional Rust closure with a default equivalent
to the reference's ` %3.0f%%`; sanitize its text, normalize line breaks and clip
at grapheme boundaries when it exceeds the component width. This deliberately
fixes reference overflow for narrow bars and invalid glyph geometry. Expose a
borrowed bar-cell traversal plus an owned structured View. Coalesce solid/empty
runs in the View; gradient/callback cells use existing rendering and fallback.
Keep literal cell/color fixtures separate from ANSI formatting and full-pipeline
performance. No API lock-in or all-module performance claim follows from this
slice; record uncovered conditions and measurements explicitly.

The historical ANSI-gradient report #143 concerns an old string-parser API,
not a reproduced defect in the pinned v2 release. Add
Gradient::from_xterm_colors for explicitly resolved canonical indexed/RGB stops:
reuse the existing xterm palette, reject unresolved default colors before
allocation, and precompute owned Lab stops directly without an intermediate
RGB vector. Actual ANSI16 palettes remain terminal-configurable. This is a
terminal-style constructor, with no image engine/codec/CUDA dependency. Require
all 256 indexed starting colors against pinned eight-sample Go gradients and
preserve the existing 548 RGB8 fixtures.
