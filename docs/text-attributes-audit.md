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

## Executed evidence at 4d1a397

All 33 native acceptance checks passed: 155 Rust tests plus two doctests,
105 Python tests, 14 snapshots and 19 Unix PTY cases. LLVM recorded 62 source
files/67 separate compilation instances. Six targeted Miri tests passed with
distinct seeds; [source and logs](evidence/attributes/miri-context.json) preserve
the reduced Miri domains. Native finite tests retain the full domains above.

[The validated report](evidence/attributes/report.json) includes all 157 legacy
controls and 171 cache-change controls. Exactly two theme Debug fixtures add
explicit new defaults during comparison; their original strings are untouched.
The four matched positive-flag construction/query fixtures use the identical
eight logical flags and are compiler-limited at about 1 ns with no allocations.
They do not establish full style/rendering throughput parity.

Balanced initial/after/after/initial repaint repeats give these medians (µs):

| Fixture | Initial pair | Color-only cache pair |
| --- | --- | --- |
| 8 variants, ANSI16 | 3.739 / 3.703 | 2.676 / 2.671 |
| 16 variants, ANSI16 | 4.211 / 4.207 | 2.670 / 2.688 |
| 8 variants, ANSI256 | 3.698 / 3.689 | 2.651 / 2.654 |
| 16 variants, ANSI256 | 4.202 / 4.193 | 2.702 / 2.671 |

Full bytes match and all six repaint fixtures request zero allocations. The
initial 8-variant ANSI16 repeat has a 7.234 µs p95; it remains in the raw data.
Truecolor results are mixed, including an initial-to-final p95 increase in the
first pass. [Every repeat](evidence/attributes/repeat-summary.json) is retained.

Legacy regressions include plain Theme construction (2.50 → 4.85 ns), style pair
adaptation at ANSI16 (17.04 → 23.19 ns), and colored full-frame output (about
5–9% higher medians across profiles). Short panel construction requests
140 → 160 bytes with the same two allocations; full panel rendering adds the
same 20 bytes. These are open optimization items, not erased by the cache gain.
Actual cell stride stays 40 bytes, while Style changes 11 → 14 bytes. A complete
new example process snapshot measures 3.559/5.004 ms p50/p95; this includes spawn
and exit and has no prior equivalent or interactive-terminal speed claim.

The new capability links 38 source candidates as partial. 6,896 source candidates,
6,902 overlapping compiler keys, 91 module groups and broader performance work
remain pending. Both strict gates remain closed; the 49 low assessments are
unchanged. Pure protocol encoders remain the sole pre-approval image exception.

[Three-OS/MSRV/reference CI](https://github.com/n3dypop/Matea/actions/runs/39091736105)
passes all seven required jobs at 4d1a397. Independent validation binds all 111
programs/127 execution inputs and 17 uploaded log hashes per host to that source.
Unix native drivers pass 33 checks, Windows 32. LLVM records 62/67 source/instance
counts on Unix and 61/65 on Windows. Each host also validates 84 reference output
cases, seven cursor cases, 91 positive Go timing-mode records, and the new
attribute/representation release steps. [Retained artifacts and source binding](evidence/attributes/ci-source-bindings.json)
keep uploaded raw and retained LF hashes distinct. CI timings are not speed data.
