# ADR 0078: Explicit color intent before shared decoration

Status: color increment qualified; full style reference axis remains open. Public APIs provisional.

The active evaluation axis is `matea-style/lib` reference. Owner direction is
one axis at a time. This increment implements the first color-input subtask;
it does not raise that axis's score or start another optimization axis.

Pinned Lip Gloss `19f011f1cc573e5d2bf9d7a9c4f14d429647759e`, `color.go`
(MIT) accepts hex/index/packed numeric notation, selects light/dark and explicit
profile colors, and exposes alpha/complementary/darken/lighten operations.
Source observations and source hash bindings live in
[the color audit](../style-color-input-audit.md). This is individual terminal
style color math; no image buffers, engine, codecs or CUDA are introduced.

Keep resolved `Color` and cell `Style` unchanged. Add separate Copy `Rgba16`
(straight channels), validated `PremultipliedRgba16` (private channels),
`AdaptiveColor` and `ProfileColors`. The existing Gradient constructor and
ADR0068's explicitly documented lost-alpha recovery remain unchanged.

`FromStr` returns a typed error rather than silently losing invalid input.
Decimal magnitude is explicitly i64-bounded on every OS, including 32-bit hosts;
reject MIN's unrepresentable positive magnitude rather than reproducing Go
signed negation overflow. Valid signed decimal notation retains the source's
absolute-value/index/low-24-bit packed semantics. Caller-selected fallback is
`unwrap_or_default()`. `Default` remains unknown terminal color; conversion and
arithmetic cannot invent known black. Indexed resolution uses the canonical
xterm palette and does not claim the user's configurable palette is known.

`Rgba16::with_alpha` changes opacity without changing straight RGB. Conversion
to a premultiplied value proves channel <= alpha, including zero alpha.
The reference Alpha helper returns `color.RGBA` with unchanged opaque channels
and reduced alpha, which can violate that type's premultiplication contract.
Preserve its raw observations, use canonical typed composition, and test this
difference. RGB8 expands by 257, fixing RGBColor's shift-only 255→65280 mapping.

Background composition is explicit `over(known_color)`, in encoded sRGB with
nearest-integer rounding. It is not linear-light mixing or an implicit terminal
background guess. u64 bounds the largest numerator; no unsafe or heap storage is
needed for an individual color. Premultiplication/unpremultiplication truncate
at 16-bit depth. RGB8 lighten/darken match the reference's channel arithmetic;
finite fractions clamp, nonfinite fractions return an error. Complementary HSV
rotation on RGB8 equals max+min-channel, avoiding float rounding/allocation.

Rejected alternatives: changing every cell to a float/alpha style; hiding
terminal I/O in color constructors; preserving invalid premultiplied states;
mixing display geometry into compact cell styles; claiming full style coverage
after adding color helpers. Geometry and full rich-style/source conditions are
the next subtasks of the same reference axis. Performance is still unqualified.
