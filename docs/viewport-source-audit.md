# Viewport source and ownership audit

Pinned Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4 exposes 70 viewport
public declaration candidates across viewport.go/keymap.go (with private
highlight helpers). [Every candidate is retained with its current partial/pending status](evidence/viewport-source-ownership/declarations.json).
The existing Matea Viewport implements a bounded vertical text slice only;
previous native correctness/performance results do not cover this full module.
The original audit introduced no declaration links or performance verdict. The
subsequent content/cell-geometry/input implementations link 50 candidates as partial;
20 remain unlinked. No whole-module verdict or quality promotion follows.

| Public function group | Existing Matea scope / remaining work |
| --- | --- |
| New/options, Width/Height, content getters/setters and line counts | Existing owned sanitized content and public rows; add replace/line input, width and complete size contracts |
| Top/bottom/past-bottom, percentages, offsets, ensure-visible, page/half-page/line/horizontal movement | Basic vertical keys only; add explicit controls, horizontal cell geometry and post-resize invariants |
| SoftWrap, FillHeight | Pending cell-aware visual row mapping and zero-width/height behavior |
| MouseWheelEnabled/Delta, KeyMap/default bindings | Pending focused mouse/vertical/horizontal key policy and configurable bindings |
| LeftGutterFunc, GutterContext, NoGutter | Pending fixed-width validated gutter with source/visual-line context |
| Style, StyleLineFunc, highlight styles | Pending styled rows, composition and clipping; use structured style/layout boundaries |
| Set/ClearHighlights and next/previous navigation | Pending checked UTF-8 byte ranges, grapheme/cell mapping and selection visibility |
| Init/Update/View, YPosition | Compose Component and typed input; audit the legacy position field's observable effect rather than assuming a runtime hook |

## Sequential public ownership observation

[The unchanged reference probe and raw result](evidence/viewport-source-ownership/context.json)
show two observable conditions. SetContentLines retains a supplied slice:
changing its first string from `tea` to a 16-column string later changes
GetContent, while ScrollRight remains limited by the cached earlier longest
line (offset 3 rather than the current text's 13 for width 3). CRLF normalization
also writes a normalized string into the caller's slice. These are sequential
public calls, not a reproduced concurrency race. The Go API does not document
a copy guarantee; the architectural improvement is explicit ownership and
cache invariants in Rust, not inventing a guarantee for Go.

Matea's current String and byte-offset index already own their content. Preserve
that boundary when adding setters: consume owned content or copy immutable
borrowed input, normalize/sanitize transactionally, rebuild dependent geometry,
clear/revalidate highlights and clamp both offsets. Do not allow external
mutation to invalidate a cached width. Share immutable configuration only when
its cache keys and lifetime are explicit.

The reference scans visual line heights in calculateLine; soft wrapping repeatedly
measures text, and gutter width is inferred by calling the formatter with an
empty context. Investigate a cached grapheme/cell index and visual-row prefix
map keyed by content, width and explicit gutter width. Do not claim a speedup
until constructor/replacement/storage costs and complete output are measured.
Validate zero/narrow widths, combining/CJK/emoji clipping, trailing empty lines,
CRLF, controls, resize, style and highlight mapping before comparing timings.
Keep the old 12 vertical-view measurements as their historical bounded cohort.

## Bounded content and vertical-control implementation

Viewport now owns normalized replacement content, consumes owned clean Strings
without copying their text allocation, accepts borrowed/owned line iterators,
and updates its index and clamped position together. Index capacity is reused.
A caller iterator panic leaves content, position and index unchanged. Content,
lines and the visible vertical slice have borrowed getters; old owned Views keep
their snapshot. Explicit height/offset, page/half-page/top/bottom/saturating line
controls share the existing geometry policy. Direct rows assignment preserves
lazy observed clamping; explicit set_height commits it.

Eleven native test methods cover these paths, including 220 pinned public
observations. Exactly 180 complete rows match. The other 40 retain an explicit
source CRLF deviation: SetContent's LF split happens before CRLF normalization,
so GetContent preserves CR. Matea preserves the structural LF policy already
used by its constructor. State/count/fraction fields still compare exactly;
source and expected content bytes are literal assertions. No raw fixture is
rewritten to hide the difference. SetContentLines does normalize embedded CRLF.
[Raw fixture hash, source references, failed first comparison and Miri limits](evidence/viewport-controls/source-review.json).

Content/replacement/control performance and new three-OS validation remain
pending; the twelve historical vertical-view timings are not reassigned.
Horizontal scrolling, wrap, fill height, mouse/key customization, gutter, styling
and highlights remain unimplemented. This is a partial capability, not F5.

A separate public caller also reproduces positive signed-overflow scrolling:
from offset 1 in four lines/height 1, Go ScrollDown(MaxInt) returns to 0. The
Matea caller uses the same positive count and reaches the saturated bottom 3;
usize::MAX also reaches 3. [Both native outputs, original callers and source/library hashes](evidence/viewport-controls/overflow-context.json).
This is a bounded arithmetic improvement, not a performance or full-module claim.

## Cell geometry and focused input extension

[The new geometry/input checkpoint](viewport-cell-geometry.md) adds explicit width,
horizontal/ensure-visible controls, constant defaults with owned key overrides
and focused wheel navigation. All 1,404 control rows match; 200 cut rows retain
161 plain matches and 39 literal deviations; 34 input states retain 33 matches
and one explicit command-wheel difference. The source edge sequence also exposes
stale horizontal offsets after resize/replacement and signed positive-movement
overflow; Matea clamps/saturates. Fifty candidates are partially linked and twenty
remain unlinked. Wrap/fill/gutter/styles/highlights and the full module performance
verdict remain pending. Historical content/control cohorts above are unchanged.
