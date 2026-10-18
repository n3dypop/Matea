# Viewport source and ownership audit

Pinned Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4 exposes 70 viewport
public declaration candidates across viewport.go/keymap.go (with private
highlight helpers). [Every candidate is retained as pending](evidence/viewport-source-ownership/declarations.json).
The existing Matea Viewport implements a bounded vertical text slice only;
previous native correctness/performance results do not cover this full module.
No new declaration links, performance verdict or quality promotion follows
from this audit.

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
