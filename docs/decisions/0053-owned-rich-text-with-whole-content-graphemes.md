# ADR 0053: Owned rich text with whole-content graphemes

Status: accepted for a narrow text/style slice; implementation/qualification in
progress. Public API remains provisional. This is not a paragraph/widget bridge.

Reevaluate the pinned Ratatui text implementation before extending the compositor,
as required by ADRs 0004/0016. Public Line/Span observations at d1ef782 reproduce
loss of a separately styled combining mark and different ZWJ/keycap cell placement
when the same sequence crosses Span boundaries. Line flattens independently
segmented spans, and zero-width spans are skipped by rendering. All eight cases,
80 raw cells, original caller, exact reference SHA and 43 matching lockfile
dependency versions are retained in evidence/ratatui-span-boundaries. This is a
buffer observation, not a claim about all physical fonts or terminal shaping.

Keep the narrow compositor and add a normalized, owned RichText document. Join
input chunks before grapheme segmentation. Store style transitions at normalized
UTF-8 byte offsets, with a base style and immutable Arc-owned data. The leading
byte of a complete grapheme selects its cell style; style boundaries cannot split
a combining/ZWJ/flag/keycap cluster. Newlines remain structural. Reuse the shared
control sanitizer, retain no caller borrow, and expose only immutable text/ranges.
Old Views keep their snapshot after application state changes.

Constant style uses the existing text painter. Multiple styles use one forward
style cursor per visible row, with a binary start lookup to skip clipped tails.
Cache intrinsic line count/maximum width once, using the compositor's per-grapheme
policy. View holds a thin Arc handle; the existing 48-byte scene, 40-byte cell and
plain-text path must retain their representation/correctness contracts.

Expose from_spans/styled constructors, borrowed text, style ranges/byte lookup,
styled grapheme iteration and View/Canvas composition. A caller iterator panic
cannot expose a half-built document. Style applies to visible cells; this does
not add layout decorators or interpret ANSI. Use wide integer comparisons before
narrowing cell widths so an oversized cluster is clipped rather than wrapped by
a numeric cast.

Require ownership/clone/thread, sanitization across chunks, canonical ranges,
cross-span graphemes, multiline/clipping/layers, old scene regressions, complete
source comparisons and allocation/distribution measurements. Do not infer parity
from reexporting Ratatui types. Its full cfg/profile graph, cell diff options,
stateful widget translation, raw-symbol handling and renderer bridge remain open.
Wrapping, gutters and highlights build on this boundary in subsequent slices.
