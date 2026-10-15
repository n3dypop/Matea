# ADR 0004 — Structured views and a narrow custom compositor

Status: Implementation decision; provisional API, review pending (2026-10-16).

## Alternatives
Ratatui's pinned `ratatui-core/src/buffer/buffer.rs:336,471` supplies grapheme-aware writing and cell diffs; `buffer/diff.rs` has substantial fixes for uncertain emoji widths. Adopting Ratatui minimizes maintenance and gives access to its widgets. It also exposes its Buffer/Style/Widget model unless translated and couples Matea layout/style to a second set of types. A string/ANSI view like Charm styling is easy to compose but makes safe clipping and extension contracts harder.

## Decision
Start with a deliberately small owned View tree (text, rows, columns, panels, positioned layers), Surface, Canvas and deterministic row diff. Use unicode-segmentation/unicode-width for graphemes and Crossterm for transport. Repaint a changed row by clearing it before writing leading cells at explicit coordinates; this is less bandwidth efficient than Ratatui's cell-level diff, but handles trailing wide-cell cleanup and avoids cursor drift. Equal rows emit nothing. Resize invalidates the frame. Transparent layers preserve lower cells except where painted; panels are opaque. Never accept raw ANSI content.

The choice is reversible behind a backend contract. Reevaluate a Ratatui adapter before adding sophisticated paragraph layout, animation optimization or a large widget catalog. No performance superiority is claimed. Ambiguous-width policy uses unicode-width's non-CJK convention; terminal disagreement remains a validation risk. Limit surface allocation and reject oversized geometry.

## Ownership
Render owns Unicode width, clipping, layout/composition and diff descriptions; style owns portable visual values; terminal owns actual commands, batching and resource cleanup. Application code never writes escape strings. Graphics resource lifetime is design-only (extension contract), with no image node/encoder implemented.
