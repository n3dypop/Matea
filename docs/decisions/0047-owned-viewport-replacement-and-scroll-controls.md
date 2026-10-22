# ADR 0047: Owned viewport replacement and explicit scroll controls

Status: implemented bounded content/vertical slice; full 70-declaration viewport module remains
pending. This extends the existing bounded vertical slice before horizontal
scrolling, soft wrapping, gutters and highlights.

The pinned public SetContentLines retains a caller slice and can normalize its
entries. A later external string replacement changes GetContent without updating
the cached longest width. The sequential public reproduction is retained in
viewport-source-audit.md; no copy guarantee or data race is invented for Go.

Retain a single owned normalized String and byte-offset line index in Rust.
Borrowed replacement copies input; owned replacement reuses its allocation when
already normalized. Line replacement consumes an iterator without retaining
references or building a temporary Vec of owned line strings. Rebuild the index
and clamp position in the same exclusive mutable call. Reuse index capacity,
keep content getters borrowed and make views own their text. Normalize CRLF and
controls consistently with the original constructor and compositor. No terminal
sequence is interpreted or emitted by this text-only component.

Expose explicit height, offset, content/count/percentage and vertical/page/half-
page controls. Use saturating usize arithmetic, preserve trailing empty lines,
and define empty/zero-height fractions. Keep compatibility with public rows:
getters clamp after direct resize; ignored events cannot commit hidden changes.
Navigation controls may normalize position because their mutation is explicit.
A one-row half page is zero, matching the reference's integer division.
Configuration remains provisional; no new crate or dependency is introduced.

Require ownership/mutation, index consistency, CRLF/control/Unicode/trailing-line,
zero height, resize and saturating-boundary tests against independent line models
and pinned public reference observations. Existing old vertical view timings
remain historical and cannot be reassigned to constructor/replacement costs.
Soft wrap, horizontal geometry, mouse/key customization, gutters, highlights,
styling and the full module performance verdict remain open.

Public observations at the pinned source expose a separate CRLF difference:
SetContent splits LF first, leaving bare CR in each line, despite its documented
normalization. Retain all 220 raw observations: 180 complete rows match; 40 CRLF
rows compare exact state plus explicit before/expected content bytes. Matea
preserves its prior normalized constructor policy for every replacement path.
Explicit set_height commits clamping, while direct rows assignment retains the
previous lazy observation behavior. Programmatic controls return observed-position
change; zero movement does not commit a hidden resize. No full viewport parity or
new performance result is inferred. See ../evidence/viewport-controls/source-review.json.

The first 9d3f9b9 content measurements identify repeated CRLF String growth and
copying: 12 requests for construction/owned snapshot and 11 for replacement/owned
snapshot. Reserve the original byte length, normalize CRLF by borrowed chunks,
reuse the shared sanitizer's borrowed result, and move the first normalized
line buffer into content. This avoids duplicating control policy or retaining
caller references. Capacity may include removed CR bytes; do not claim a resident
memory improvement. Full source-bound repeats and adverse controls are retained
separately; whole-module parity/performance remains pending.
