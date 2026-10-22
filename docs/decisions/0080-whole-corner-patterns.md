# ADR0080: Whole corner patterns with cached rectangular extents

Status: isolated prototype,not integrated or qualified as current Foundation.
The active evaluation axis remains matea-style/lib reference50. No score change.

Lip Gloss issue605 requests multiple independent graphemes per corner; its pinned
renderer truncates each corner to one rune. Issue114 reports a wide right corner
extending beyond shorter body rows. At19f011f1 the empty-body probe renders the
middle row narrower,while Width(5) around `ab\nc` produces width4 because getter
extents and rendered extents disagree. The source observer is frozen separately.

Keep FirstCluster as the default and add explicit CornerMode::WholePattern.
The full mode validates and caches the sum of the existing grapheme widths for
each corner with checked u16 addition. It shares immutable BorderSet storage;
changing the mode of a clone never changes the original. Edges retain their
maximum-cluster width. Default corner extents use the cluster actually drawn,
not an unused later glyph in a truncated pattern. Empty parts remain0 in width
queries,with a one-cell space fallback when an enabled corner is drawn.

Reserve complete corner widths in the adjacent sides. Paint left edge patterns
against the outer left boundary and right patterns against the outer right
boundary,with separately styled blank reservation cells. This keeps every row
rectangular even when a side pattern alternates widths. Whole-pattern corners
paint their cached borrowed glyphs with a color sample at each glyph coordinate;
no resegmentation or temporary per-corner color vector is needed. Whole-grapheme
clipping,body style ownership and border gradient offset direction are retained.

BorderParts provides borrowed named fields as an ergonomic custom-border input;
the existing13-part array remains accepted. BorderSet construction owns validated
strings and segmentation before returning. No borrowed input is retained in a
View,Style or Surface. Cell/Style representations do not change. API remains
provisional;no public naming/registry availability claim is made.

An isolated native prototype passes existing3573 source observations and new
independent literal rectangles,combining/CJK/ZWJ crop cases,all corner gradient
coordinates,signed offset extremes,65535/65536-cell width boundaries,borrowed
input ownership and mode/iterator lifetime tests. These are correctness/design
checks,not benchmark wins. Integration,native/Miri coverage and3OS qualification
are required before adopting it or closing the two issue assessments.
