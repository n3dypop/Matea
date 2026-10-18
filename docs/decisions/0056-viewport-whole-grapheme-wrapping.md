# ADR 0056: Whole-grapheme viewport wrapping

Status: implemented; qualification and performance review in progress.

Add soft wrapping and fill-height to the existing owned viewport. Keep logical
line access stable; display_line_count and vertical positions refer to physical
rows while wrapping is active. Require an explicit width to compute geometry;
clearing width suspends wrapping until sizing resumes. Zero width yields zero
display rows and zero positions without floating point/division.

Wrap at complete EGC boundaries with the compositor's width policy. A cluster
too wide for an otherwise empty row becomes one U+FFFD display placeholder,
retaining original content for inspection/resizing. Preserve structural/trailing
empty lines and ignore standalone zero-width items in painting as before. Do
not split a wide cluster across rows or let emitted cells exceed the row budget.
This intentionally improves fixed cell cuts that can discard/duplicate content
or insert extra newlines. Fill-height adds borrowed empty rows to inspection;
explicit zero width/height remains empty. Existing default flat rendering keeps
its direct borrowed-content path.

Cache UTF-8 row starts, not owned strings per row, behind OnceLock<Arc<_>>. Only
oversized clusters need an additional sorted list of row indices. All getters
remain Send/Sync, clones can share immutable geometry, and changing width/content
or the wrapping mode invalidates only that viewport's slot. Private byte starts
derive from whole-grapheme boundaries. Positions preserve their numeric display
row across reflow and clamp to the new range; they do not promise anchor tracking.

ensure_visible continues to accept a logical line and cell range, mapping the
start to a display row. Invalid ranges remain transactional. Enabling wrap resets
horizontal position; horizontal navigation is disabled. visible_text borrows the
original source range and inserts no soft breaks; visible_lines exposes actual
display rows and placeholders. The first geometry query can allocate the cache.

At pinned Bubbles c3bde0d, a 1,008-case native public-source matrix (7 corpora,
6 widths, 3 heights, fill on/off, 4 actions) retains 586 exact canonical states
and 422 literal deviations. Complete source View bytes are preserved before
right-ASCII-space trimming. Deviations cover zero-width count overflow, logical
ensure mapping, complete-cluster wrapping/oversized replacement and per-EGC
script width. Zero-width Go float-to-int behavior is platform dependent: CI must
retain those observations separately, and compare portable nonzero cases exactly.
Do not normalize invalid signed source totals into a claimed match.

Eight native integration tests cover geometry, full glyph stream/cell budgets,
empty sizes, fill, normalization/ownership, concurrent clones, logical ensure,
and every literal matrix state. Cross-platform/Miri, matched distributions,
styles/gutters/highlights and full module completion remain pending.
