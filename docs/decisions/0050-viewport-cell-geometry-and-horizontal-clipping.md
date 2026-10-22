# ADR 0050: Viewport cell geometry and horizontal clipping

Status: implemented bounded geometry slice; public APIs remain
provisional. Full viewport wrapping, styles, gutters and highlights are pending.

Keep the default parent-sized vertical viewport. An explicit optional `u16`
width enables horizontal navigation; `Some(0)` is an empty viewport rather than
an inferred parent width. Explicit resize commits clamped offsets. Clearing
width disables horizontal navigation and resets its position. Saturating `usize`
movement cannot wrap at machine limits. Validate inverted ensure-visible column
ranges before changing either position, and return a typed error.

Measure logical line width with the compositor's policy: ASCII bytes or the sum
of Unicode grapheme widths, skipping zero-width clusters. Do not use whole-string
width when the compositor measures per grapheme. Cache only the longest width in
an owned `OnceLock<usize>`; the standard synchronization primitive preserves
Send/Sync and Clone without custom atomic sentinel encodings. Content replacement
invalidates the cache under exclusive mutable access. A vertical-only workload
does not initialize it. Do not add a per-cell geometry vector before measuring
its construction, storage and invalidation costs.

Provide a borrowed/lazy visible-line iterator, returning Cow only where a partial
wide grapheme at the left edge requires space. ASCII and aligned Unicode slices
borrow the original content. The compositor remains responsible for cells and
styles; text input remains sanitized, with no interpreted ANSI escapes.
Never draw half a grapheme: replace the visible remainder at a cut left boundary
with spaces and omit a partial grapheme at the right boundary. A later complete
grapheme retains its original cell position. This intentionally improves the
pinned x/ansi Cut policy, which retains a whole wide grapheme across the left
boundary. Verify the public reference output and retain literal deviations.

Preserve the previous focused vertical-key behavior. Add left/right key handling
only with explicit width and a positive configured step. Release and command
modifier events remain ignored. Configurable full keymaps and mouse dispatch
will use their own tested slice; this change does not claim them.

Require independent cell-window and position models, content/cache/clone/resize
invariants, large inputs, CJK/combining/emoji/zero-width boundaries, unchanged
vertical fixtures, public reference controls and literal clipping deviations.
Measure complete outputs with construction/replacement costs distinguished from
borrowed getters and owned rendering. No full module parity or maximum speed is
inferred from these bounded cases.
