# ADR 0052: Bounded viewport width and default input fast paths

Status: implemented; both finite corpora and final native/Miri/LLVM pass.
Three-host qualification is reported separately.

The complete first 14-workload cohort exposes about 69 ns for default Right
dispatch versus 20 ns for wheel, and 26 microseconds for a repeated Unicode
document's construction. A second baseline uses the unchanged library with both
repeated and unique per-line suffixes before applying the optimization.

Select a candidate default action directly from the typed KeyCode, then validate
the event against the canonical constant chord group. Keep modifiers/help in
that one canonical definition. Explicit overrides retain the prior full ordered
lookup and precedence. Release exits before lookup. This avoids serially testing
every default action while preserving public matches/chords/help behavior.

When the entire normalized content is ASCII, derive longest line length directly
from the owned LF index and tail. Otherwise remember only the previous borrowed
line while scanning; equal adjacent lines have equal widths and need no repeated
Unicode segmentation. Distinct lines retain the original per-grapheme policy.
This adds no hash table, persistent text cache or allocation. It deliberately
avoids assuming that an undocumented byte-length bound or a whole-string width
metric is interchangeable with the compositor's policy.

Keep the completed viewport in its private module with root reexports, matching
the other component modules. Move its owned geometry/cache/input/render helpers
and private cache unit test together. The byte-for-byte extraction proof precedes
rustfmt; source-bound native/Miri/CI evidence qualifies the final module. This
organization change has no independent claimed performance benefit.

Validate every existing pinned control/cut/input fixture plus repeated/varied,
empty/trailing, ASCII-index and mixed-width replacement cases. Retain full
before/after distributions for both corpora and adverse controls. Construction,
owned plain consumption and borrowed iterators remain separate; no full viewport
or Foundation performance verdict follows from this optimization alone.

The repeated Unicode construction drops from about 26.5 to 7.2 microseconds and
default input from about 65 to 26 ns in both orders. Varied Unicode construction/
replacement shows observed p50 increases of 0.3–1.3%; small controls and adverse
tails remain in the report. No full-module/RSS/maximum-speed claim or quality
threshold change follows. See ../viewport-cell-geometry.md and its raw comparison.
