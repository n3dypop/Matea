# Matea design system

Theme is a passed value, not a global. The default palette evokes matcha and dark tea ceramics. `Theme::plain()` keeps ASCII borders and removes color while retaining bold, reverse focus and underline errors. `RunOptions` accepts a custom Theme and explicit Capabilities.

| Token | Purpose | Non-color cue |
| --- | --- | --- |
| text / surface | Content and panel background | Plain readable labels |
| accent | Title, header, primary action | Bold |
| muted | Help and supporting text | Position and explicit wording |
| border | Panel boundary | Rounded/plain/ASCII glyphs |
| focus | Current input/selected item | Reverse + bold, visible `>` marker |
| error | Validation failure | Underline and `Error:` prefix |
| gap / padding | Inter-component and panel spacing | One cell by default |
| reduced_motion | Motion preference | Static `* Working` spinner; app stops timer |

Styles are complete values; they do not inherit an ambient terminal color. App authors use semantic tokens rather than ad-hoc ANSI. Foreground/background/underline colors adapt to None/ANSI16/ANSI256/TrueColor using deterministic nearest-palette conversion. A test checks at least 4.5:1 computed RGB contrast for text/muted/accent/error against the default surface. Actual terminal palettes can be customized; no color-contrast guarantee is inferred for every terminal.

Individual style colors accept fallible hex/index/packed numeric parsing.
AdaptiveColor and ProfileColors select explicit theme/profile choices without
terminal I/O. Rgba16 holds straight channels; PremultipliedRgba16 validates its
different alpha contract. Resolve the background before composing transparency;
unknown Default never becomes black. See [color input contracts](style-color-input-audit.md).

Layout uses columns, equal/fixed-width rows, panels and ordered positioned layers. Text clips; it does not wrap. The dashboard demonstrates a shared theme, table and viewport with the same spacing/focus/help language. Its reviewed plain golden is `tests/golden/dashboard.txt`. Native rendering uses rounded borders when Unicode is allowed; TERM=dumb selects ASCII and no color, and NO_COLOR disables color without removing attributes.

Screen-reader support is limited to plain snapshots. Interactive linear prompts, real emulator comparisons, user testing, contrast verification against user-customized palettes and IME composition are open acceptance risks. Do not market the Foundation as accessibility certified.

TextAttributes packs independent flags and exclusive underline/blink/font-frame
choices behind typed setters. Resolved styles can request dim, italic, strike,
conceal, Fraktur, overline and font framing without emitting escape sequences.
The cell renderer keeps each whole grapheme and its continuation cells styled
together. The terminal layer suppresses hardware blink for reduced motion and
resets complete style state when decorations are removed. Font decoration support
is terminal-specific; focus and error cues also use text/position.
