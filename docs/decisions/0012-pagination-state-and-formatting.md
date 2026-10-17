# ADR 0012: Safe page state, reusable navigation and Rust formatting

Status: implementation decision; public API remains provisional.

Bubbles paginator exposes mutable Page, PerPage and TotalPages. Its fixed source
`paginator/paginator.go:63–120` can divide by zero, return reversed slice bounds
for a stale page and retain a nonempty page count when asked to clear all items.
These are source-derived edge cases, not maintainer-confirmed issue claims.

Keep Pagination as a Copy value with private invariants. Item-backed extents
recompute the count on a size change; explicit page-backed extents support document
and virtual pagination independently of collection length. Per-page is NonZeroUsize.
Every mutation clamps the active page. range_for(length) saturates the offset and
clamps both endpoints to the actual collection. Empty content is explicitly 0/0;
clearing item count clears page count. An unrepresentable virtual offset gives an
empty range at length, never an invalid slice.

Compose Paginator from this state, typed bindings and themed Views. Defaults match
the reference's keys, one-page Arabic label and dot glyphs. Focus, repeat, release,
disabled binding and help policies are shared with the other components. A matched
boundary key is consumed without emitting Changed. Next binding takes precedence
if a caller assigns overlapping chords, matching the source dispatch order.

Use a UI-thread formatter callback taking PageNumbers rather than interpreting Go
printf syntax. Applications use format! (including width, padding, radix and
localized labels). Rc shares immutable callbacks across component clones without
requiring Send on UI-owned state. The default path has no callback allocation.
Glyph strings and labels are plain content; styling uses Theme/View rather than
embedded terminal escapes. Callers can compose a custom View for richer labels.

Bound dot-node construction by an explicit budget (default 80). A centered,
edge-clamped window always includes the current page when the budget is nonzero.
This extends the earlier first-N truncation, which could hide the active dot.
Unbounded full output remains available by supplying a sufficient budget; measured
comparisons use identical complete dot labels and do not include View construction.

Source: Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4,
`paginator/paginator.go`, `paginator/paginator_test.go`. See the module audit for
declaration mapping, executable source-edge reproduction and performance evidence.
