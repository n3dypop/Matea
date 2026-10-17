# ADR 0018: Borrowed, width-bounded key help

Status: implementation decision; API remains provisional.

Provide short/full key help as composition over binding metadata, independent of
runtime internals. Borrow keys/descriptions for each view call instead of cloning
bindings and their key vectors. A HelpMap contains short items and full groups;
Help::view selects the configured mode, while explicit short/full methods are
available. Disabled/unbound bindings are excluded by the same help contract as
matching. Applications regenerate borrowed items after state changes.

Use one layout algorithm for plain text and structured styled views. Keys,
descriptions, inter-item/group separators, ellipsis and the key-description gap
have independent styles. The gap addresses Bubbles report #571 without inserting
terminal escape strings. Defaults use Matea theme roles; explicit light/dark
defaults and caller-supplied styles remain available. This is an original visual
system, not an exact copy of the reference palette.

Width is Option<u16>: None is unlimited; Some(0) deliberately hides output during
zero-width resize. Fit complete items/columns and add a whole ellipsis only if it
fits. Never accept an overflowing item merely because the ellipsis cannot fit.
Measure sanitized text in terminal cells and preserve extended graphemes. Inline
metadata maps newlines to spaces; terminal control sequences are never executed.
Full help aligns key and description columns at the top, omitting disabled groups.

Inspect pinned Bubbles help/help.go at
c3bde0d166c2ca607d9864493453fd77bdcae7e4, including shouldAddItem's overflow branch,
and keep source reproductions distinct from maintainer-confirmed defects. Match
ordinary unstyled fixtures before performance comparison; intentional width/safety
deviations require separate regressions. Complete module review remains pending
until correctness, declaration links and finite performance evidence are recorded.
