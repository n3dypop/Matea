# ADR 0059: Validated byte highlights and cell-style composition

Status: narrow implementation; full qualification and performance pending. Public API remains provisional.

Pinned Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4 implements search
highlights in viewport/viewport.go:597–645 and viewport/highlight.go. It accepts
ordered, nonoverlapping UTF-8 byte matches, converts them into per-line maps and
styles ANSI strings before clipping/wrapping. Selected ranges are styled a second
time. Invalid inputs are assumed away. Empty match input leaves existing matches
unchanged; logical-to-display navigation and interior-EGC matching also have
observable limits. These observations come from the actual frozen source caller,
not a claim about every version or an open issue's current status.

Use normalized `Viewport::content()` byte ranges, not terminal escape offsets.
Validate ordering, bounds and scalar boundaries before publishing any state;
errors identify the rejected input index. Iterator panics preserve the old
selection and scroll position. Range validation does not require EGC boundaries:
searches can legitimately match a combining scalar or an emoji constituent.
Style every whole EGC intersecting a nonempty match. When several disjoint matches
intersect one EGC, selected intent takes precedence. Empty ranges remain navigable
and paint nothing. Empty input clears all matches. Replacing content clears stale
geometry, including when the replacement has the same byte length.

Store an immutable Arc-owned vector of ranges/first-line anchors, with selection
local to each cloned viewport. A monotone grapheme cursor computes anchors without
allocating a map per match or rescanning every prefix. Reuse the existing lazy
wrapped byte index to select/reveal matches in display rows. No-selection previous
navigation selects the final match. Top/bottom update the nearest selection;
ordinary scrolling preserves it. Long multiline matches reveal their first
logical row/range, since the whole match cannot always fit in a window.

Resolve `highlight_style` against theme text, then apply selected StylePatch intent.
An explicit false/default disables a property; unspecified properties inherit.
This preserves the ordinary highlight under a selected reverse/background patch.
Matea's default underline plus selected reverse is a deliberate visual default;
callers may supply empty patches. Do not emulate source ANSI reset nesting by
silently dropping inherited attributes.

Render visible source intervals as RichText spans using the same clipping bounds
as the plain pager. Generated left padding uses base style; oversized wrapped
replacement glyphs carry their source range's intent. LF and fill padding paint
no phantom match cells. The inactive path keeps the existing plain view and
creates no match storage. Active construction/rendering allocates; do not claim
zero cost or constant-size memory. Geometry creation is linear in the source
prefix and match count; active rendering currently searches sorted matches per
visible EGC. Full-document residency and large/varied workload costs still need
measurement. No ANSI parsing/string emission enters the widget API.

The original source caller retains 420 conditions with raw ANSI and interpreted
lead cells; 252 ASCII cases additionally pass independent strict SGR replay.
190 paired states match; 230 have literal Matea expectations and explicit reasons
for changed geometry, matching or style composition. Wide placeholder style is
not an independent drawn SGR request and is canonically zeroed on both sides.
This is not 420 matches or whole-module equivalence. Six matched styled frame
workloads use whole-EGC, flat documents and an explicitly complete source selected
style so the cell outputs, positions and full FNV projections agree. Shared-runner
checks establish reproducibility, not speed ratios. Require native/Miri/three-OS
qualification and retain adverse distributions before evaluating performance.

Line-style callbacks, fixed gutters, arbitrary viewport borders/margins/padding,
rich input/ANSI text preservation and complete widget/reference coverage remain
separate open capabilities. Stage A and the F5 owner gate remain closed.
