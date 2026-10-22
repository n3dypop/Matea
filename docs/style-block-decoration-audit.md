# Structured block decoration: active style reference axis

This page records the qualified f9c34b7 checkpoint. The subsequent corner
candidate is tracked in[corner-pattern-audit.md](corner-pattern-audit.md).
This is a bounded increment of `matea-style/lib` reference, still score50.
The47 below-high axes and threshold75 are unchanged. No additional axis closes.
The owner authorized approximately25 sequential axes; a decorator subtask is not
an axis. Full source/conditional exports, hyperlinks, precise/custom transforms
and the remaining style-family capabilities still prevent this axis closing.

## Ownership and API

`Style`/`StylePatch` stay compact Copy cell values. `BlockStyle` holds optional
layout intent: None is unset; explicit0/false/terminal-default survives
inheritance. `Insets` use cells, `Edges` give typed directional access, and
`BorderSet` caches owned custom graphemes once, sharing them on Clone. Built-in
presets use static strings. No mutable borrowed strings, ambient renderer or
terminal handles are retained. All production code is safe Rust.

`matea-render::StyledBlock` owns normalized `RichText`, applies a capturing
thread-safe transform once, expands/removes tabs explicitly, wraps complete
clusters, decorates and produces a Surface/View. Rich input retains its style
runs; transformations on rich input require explicit application by the caller.
Its viewport limit is applied before Surface allocation. Layout rows refer to
shared byte ranges rather than allocating one styled object per character;
painting stops at the visible horizontal boundary. These are design properties,
not measured speed or allocation wins. Performance remains unmeasured here.

[ADR0079](decisions/0079-structured-block-decoration.md) records dimensions,
clipping, custom border validation and deliberate source corrections. New types
are provisional and exported by the facade prelude. Existing Panel remains
available without changed rendering behavior.

## Fixed source observations

Lip Gloss `19f011f1cc573e5d2bf9d7a9c4f14d429647759e` is the decoration reference.
Ultraviolet `6b8d4baf91b4c933e6613b0eefc8f180f6eda002` projects its ANSI output
into cells using x/ansi `faa4adf95555331dfb439b716cc83767ecdadeb4` (v0.11.9).
The frozen producers under [block-style evidence](evidence/block-style/) call
those actual APIs. Fixtures preserve raw ANSI, dimensions and cell metadata;
they are not Rust-generated golden outputs. Pure geometry/cell metadata is
compared, without implying emulator or terminal hyperlink interoperability.

| Corpus | Observations | Criterion |
| --- | ---: | --- |
| Built-ins,16 side masks,wrap,padding,margin,alignment,dimensions,clipping,attributes,tabs,fills |2165|2123 exact source cell grids;42 explicit corrections |
| Border gradient:3 stop sets,16 masks,dimensions,positive/negative offsets |960|Exact source dimensions/colors/cells |
| Custom ASCII edge patterns and multi-character corners |96|Exact source cells; first complete corner glyph |
| Inheritance:8 attribute sets ×8 child override combinations |64|Exact source cells, explicit reset/false/zero and child spacing |
| All6 underline groups,blink/faint,space controls,combining-space leading style |288|Exact source cells;144 initial combining-space mismatches repaired |

The complete native corpus is3573 observations,3531 exact and42 independently
modeled corrections. Model expectations are literal corrected content followed
by independently calculated geometry; they never call the production wrapper,
block layout or align helper. A stale exception fails if it no longer differs
from the source. No mismatch is ignored or relabeled parity.

The42 corrections are36 narrow-wrap observations (whole combining/ZWJ clusters,
hyphen word boundaries,overflow whitespace),5 inline observations (no subtraction
of a border that is not drawn),and1 wide padding/margin observation (cell units,
not repetitions of a wide character). Inline disables line breaks,wrapping and
frame decoration; explicit width/height minima still apply. Its known min-height
behavior is retained; calling it inline does not erase explicitly requested
blank rows. A one-cell fill cannot accommodate a two-cell glyph and uses a space.

Source `borders.go:498` cannot advance its edge loop for an all-zero-width pattern.
A separate frozen observer expires a50ms watchdog; this is liveness evidence,
not a timing comparison. Its rune corner truncation drops combining marks and
its rune side cycling splits ZWJ emoji. Matea rejects zero-width/control patterns
before publishing a BorderSet, keeps whole clusters, reserves each side's maximum
width and fills an unfit remainder. Literal wide/corner regressions are separate
from the exact ASCII corpus. Source fill characters *do* inherit correctly; the
observer verifies46 and33 for `.` and `!`,rejecting the earlier omission hypothesis.

## Independent tests and qualification

Generated input uses all3125 five-token combinations of ASCII letters,hyphens,
spaces,CJK and combining clusters at six widths (18750 renderings). It checks
cell width bounds and conservation of non-whitespace content. This found actual
Matea defects in pending-hyphen/space handling; failed runs remain in evidence.
Other tests cover invalid/control fills and borders,explicit inherited intent,
shared ownership,user panics,rich style/cluster merging,whole-grapheme crop,
View composition,zero viewport,large checked dimensions and bounded allocation.

Current-source47 native commands pass:Rust306 tests+17 doctests and151 Python
tests(two conditional skips). Miri seed241 passes4 style+11 render methods;
49 selected source rows,150 generated renderings and direct boundary tests run.
The42 correction rows run natively. LLVM reports102 paths/109 separate compiler
instances; its3 zero-hash transitive hashbrown warnings are diagnosed and retained.
New production line execution is98.72% block/99.29% decoration(includes embedded
tests in decoration); these are not branch or memory-safety proofs. Native
[Linux/macOS/Windows+MSRV CI](https://github.com/n3dypop/Matea/actions/runs/39746958503)
passes all four required jobs at f9c34b7. All212 execution inputs at that checkpoint match
that commit exactly; each OS executes47 acceptance commands and all15 new
style/render methods. Uploaded raw journals/logs are hash verified; missing
upload members are explicit. Reference/measurement-driver recollection is skipped.
[Qualification](evidence/block-style/qualification.json) does not close this axis. No benchmark was performed,
no performance axis promoted,and no image engine/codec/CUDA work started.


[Issue522](https://github.com/charmbracelet/lipgloss/issues/522) does not reproduce
at the fixed source:implicit and explicit normal borders both report2×2 overhead
and render3×3 around `a`. The actual observer and source getter hash are retained.
[Issue114](https://github.com/charmbracelet/lipgloss/issues/114),
[issue605](https://github.com/charmbracelet/lipgloss/issues/605) and
[issue731](https://github.com/charmbracelet/lipgloss/issues/731) remain partial:
only-wide-right-corner alignment needs a dedicated regression,multiple separate
corner graphemes remain truncated,and native32-bit execution is unverified.
The current issue ledger counts821 pending reports; this is a pinned-source
assessment,not a GitHub issue closure. No messages or issue changes were sent.
