# ADR 0079: Separate block decoration from resolved cell styles

Status: decorator increment implemented and qualified at f9c34b7; API provisional.
[Qualification](../evidence/block-style/qualification.json). Active axis remains
matea-style/lib reference; no other axis is promoted by this increment.

Keep the compact Copy Color/Style representation unchanged. Add BlockStyle,
Edges, Alignment and immutable validated BorderSet to matea-style. They contain
explicit optional intent rather than an ambient style or terminal state.
Public fields and typed builders replace duplicate Go setters/getters/unsetters;
None means unset, Some(false/0/default-color) remains an explicit override.
Inherit fills absent properties while preserving the child's padding/margin.
Text and transform ownership stay outside cells.

matea-render owns StyledBlock: normalize input after optional transformation
and explicit tab conversion, measure/wrap whole graphemes, lay out padding,
minimum dimensions and alignment, then borders, margins and final clipping.
It yields an owned Surface/View snapshot without emitting ANSI or acquiring a
terminal. Applications reconstruct it from the current ViewContext on resize.
No additional render→widget/terminal dependency or crate cycle is introduced.

Lip Gloss19f011f1 color/style/borders/get/set/unset/align files are the pinned
behavioral reference. Built-in border glyphs retain their source values. Custom
border patterns are validated once, owned immutably, and cycled by grapheme.
Reject controls, structural newlines and zero-width graphemes. The source's
horizontal loop otherwise cannot advance for zero-width patterns; its rune
corner truncation and repeated rune cycling can split combining/ZWJ clusters.
Rust preserves full clusters and fills an unfillable wide edge with spaces.
Top/bottom borders consume a row; left/right reserve their maximum glyph width.
Custom wide-border source geometry differences require explicit regression
fixtures; they are corrections, not silent source equivalence.

Dimension arithmetic uses usize with checked sums and the existing1,048,576-cell
Surface bound. A viewport limit bounds allocation before drawing. Large minimum
height/width must not allocate an intermediate unbounded canvas merely to crop
it later. Margins and padding are distinct styling roles; borders cannot leak
body attributes into adjacent cells. Whole-grapheme clipping remains mandatory.

New executable examples, pinned source fixtures, independent geometry/negative
models and native/Miri/3OS qualification are required. Shared decorator behavior
must be implemented before Help/Cursor/Spinner/Progress/TextInput reference axes
can close. Full links, transforms, arbitrary precise colors, source conditional
exports and performance remain explicit obligations of the active style axis;
adding a subset of geometry does not complete it. Foundation/F5/image gates
remain unchanged.

The narrow wrapper preserves source breakpoint behavior where it remains
bounded. Full pending words are flushed before they can cross the wrap limit;
line-boundary overflow spaces are dropped. Literal narrow fixtures preserve
whole clusters rather than the source's ASCII/UTF8 parser split. Inline removes
structural line breaks and frame decoration, retaining explicit width/height
minima without subtracting hidden border extents. Wide margin/padding fill uses
cell counts, filling a short remainder with a space. These intentional
corrections are independently modeled and counted separately from exact source
observations. Gradient offsets rotate in the source's signed direction;
sampling is lazy with no full perimeter color allocation.
