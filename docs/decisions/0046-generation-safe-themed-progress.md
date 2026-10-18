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

Initial source a51f9a3 passes all 33 native checks, 178 Rust tests plus two
doctests, 115 Python tests, 16 snapshots, 21 Unix PTY cases and LLVM collection
(71 files/76 compiler instances). Targeted Miri executes eight progress widget,
two facade/runtime/fallback and four gradient tests, including all 120 progress
fixtures plus the existing finite gradient subsets. All six complete frame
outputs/first returns match. The first two balanced performance pairs are
retained, with observed p50 at 4.6–23.2% of the reference for these finite cases.
No constructor/scheduling/whole-module verdict follows from those frames.

The separate cell controls reveal two requested heap operations for the default
percentage label. Replace its private String with existing CompactString inline
UTF-8 storage and format_compact, preserving public borrowed-str access and
custom formatter signatures. This adds no dependency. Keep the a51f9a3
native/Miri/performance cohort, and rerun validation and the complete balanced
measurement for the changed source. A long Unicode custom label must still
exercise heap ownership and grapheme-safe clipping; no blanket zero-allocation
promise applies to custom formatters.

The inline-label Miri run (seed 211) exposes an overly strict test assertion:
two independently constructed springs differ by about 2.2e-16 in position and
1.3e-15 in velocity. Preserve the failed log/context. Compare only that
independent one-step/cadence calculation within 1e-12 absolute error; retain
exact equality for ignored/stale ticks and exact final snap. No fixture output
or reference trajectory tolerance is relaxed. Rerun the same Miri seed after
this test correction. Also upload the new progress/indexed fixture outputs and
timing-mode checks so each host's results can be independently inspected.
