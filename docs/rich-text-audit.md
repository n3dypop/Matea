# RichText: owned text and whole-content graphemes

Implementation/qualification in progress. F5 remains closed. This adds a narrow
structured text boundary required by later viewport styling/highlights; it does
not complete the Ratatui text, paragraph or renderer API.

The public pinned ratatui-core Line/Span source at
`d1ef782b48ddf0cb825c111d4d8b9adef572c0e1` segments individual spans. The native
80-cell observation retains four aligned matches and four cross-span deviations:
combining mark, ZWJ, regional-indicator pair and keycap. RichText joins normalized
owned text first, then segments complete graphemes and selects their leading
byte's style. Full literal source/expected observations are regression fixtures;
different continuation-cell storage is explicitly accounted for. This is a
buffer comparison, not physical font or terminal shaping evidence.

An immutable Arc shares content/styles/cached intrinsic geometry; no borrowed
caller storage survives construction. Style ranges use normalized UTF-8 offsets.
Uniform styles reuse the ordinary painter; mixed styles traverse one forward
cursor per visible row. View remains 48 bytes and Cell remains 40 bytes. Input
controls follow the existing sanitizer. Iterator failure cannot expose a partial
document. See ADRs 0053/0054 and the nine native integration tests.

The private excluded `tools/reference-text` workspace locks the exact Git source,
std-only profile and all 43 reference package versions. Its original caller code
contains no vendored upstream source. Additional Matea dependencies are recorded
separately in `evidence/rich-text/locked-reference-context.json`; root Cargo.lock
is unchanged. Direct root dev-dependency unification changed fifteen reference
versions, so that experiment was removed instead of accepting shifted conditions.

The new benchmark has ten matched cases: owned construct plus full document read,
immutable clone plus read, width lookup, styled grapheme traversal and fresh-frame
render plus read, each for ASCII and Unicode. Twelve grapheme-aligned owned style
chunks and every visible leading cell/blank padding are consumed. Full independent
text/style/range/frame strings and rolling digests are checked; the native oracle
also compares literal grapheme lists. Cross-span deviations are correctness
controls, excluded from speed ratios. Render includes Matea View/Arc construction.
Timing, allocations, native/Miri/LLVM and three-OS qualification are pending.

Mutable span fields, style patch/reset algebra, alignment/conversion families,
CJK width, borrowed-storage APIs, full paragraph/wrapping, physical emulators,
all feature profiles and complete module performance are still pending. No high
performance verdict or whole-module parity is inferred from this slice.
