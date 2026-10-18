# Resolved text attributes

ADR 0041 adds Copy resolved attributes owned by matea-style. Component code has
no dependency on a terminal backend. The attributes example uses theme tokens,
an explicit focus marker and ordinary Model/update/view, with no terminal setup.
Private u16 storage prevents invalid underline/blink/frame choice combinations
through safe public construction. No allocation or unsafe conversion is needed.

The pinned Lip Gloss set.go/get.go/style.go at
`19f011f1cc573e5d2bf9d7a9c4f14d429647759e` requests bold, faint, italic, reverse,
blink, strike and six underline choices/colors. Crossterm attribute.rs at
`9a397df6dc4f5bd3ff900bb6308b728186cb914c` adds conceal, Fraktur, overline and
font framing. Its positive attribute command outputs are the finite byte oracle.
The registry dependency is 0.29.0. Its attribute enum is equal after CRLF
normalization; the bitset file differs only by the pinned derived Hash, and both
SetAttribute/SetAttributes write_ansi bodies match exactly after normalization.
The complete style.rs files differ in other features, so no full-repository
identity is asserted. Raw hashes and the narrower command checks are retained in
[reference-source.json](evidence/attributes/reference-source.json).

Native tests enumerate 13,824 valid states, check independent flags/choices and
clear back to default, and compare all 13,824 positive SGR combinations with
Crossterm. Literal fixtures cover indexed/RGB underline headers and reset on
removal. Existing exhaustive color tests independently check nearest-palette
conversion; underlines now share those fallback paths. Clipping keeps combined
and wide graphemes intact with their styles. Reduced-motion encoding removes
blink while preserving other decorations and repaints when capabilities change.
These are command and logical-cell tests, not physical emulator/font validation.

The new matched micro-workloads construct/query the same eight positive flags
with both Matea and Crossterm, with all 256 masks checked outside timing. They
exclude choice attributes, serialization and style inheritance. Six additional
Matea full repaint workloads vary 8/16 attribute sets over a shared color triple;
literal color bytes and reference SGR check the entire output. These expose
palette-cache interference caused by changes unrelated to color. No full
Lip Gloss/Ratatui/terminal throughput claim follows from this subset.

The old full-style palette key recomputes expensive color conversion when more
than eight attribute variants evict each other despite identical colors. Capture
that implementation before changing the key; a proposed color-only key must
reattach the caller's original attributes and preserve all output/fallback state.
Use actual distributions and separate allocation counters for acceptance.

Remaining: resolved defaults do not encode Lip Gloss's unset/inherit distinction.
Transforms, space-specific decoration, rich text/layout, borders and arbitrary
style composition remain pending. Crossterm negative/reset command sets and raw
numeric flag interoperability are not the same contract as resolved Style. New
builders do not imply complete public module coverage or guaranteed terminal
support. Style layout remains compiler-chosen and is not a stable ABI.
