# ADR 0013: Grapheme input, shared editing paths and bounded display

Status: implementation decision; provisional API, not a complete TextInput module.

Replace the old full-value clone and boundary Vec on every input event with a
mutable UTF-8 buffer, cached grapheme count/ASCII state and borrowed boundary
iteration. ASCII cursor moves are constant-time; insert/delete reuse String
capacity. Unicode mutations recount actual joined clusters and repair the cursor
to a whole-grapheme boundary. Byte indices stay private; public position/set_cursor
use grapheme units. Word commands use whitespace-delimited grapheme sequences.

The input budget has two independent units: extended graphemes (default 1024) and
retained bytes (default 64 KiB). The byte limit also bounds a cluster containing
many combining marks. Explicit setters truncate whole clusters immediately;
direct public budget assignments affect future input. usize::MAX can remove a
budget. A combining/ZWJ insertion can fit a full grapheme budget; check the joined
result rather than rejecting it solely on the isolated inserted cluster count.
Trim excess inserted clusters without discarding the original suffix.

Single-line input borrows clean text; newline/CR/tab become spaces and other C0/C1
controls are removed. A legitimate U+FFFD is preserved because Rust str already
guarantees valid UTF-8. This is distinct from rendering, which displays replacement
markers for controls in arbitrary application content. No terminal escapes are
interpreted by the widget.

Typed TextEditKeys provide movement, word editing, deletion and suggestion keys.
Use the shared down-event/release and enabled-help policy. Focus remains an explicit
Component argument controlled by the parent; do not duplicate it inside the leaf.
Forms first offer Tab to a completing child, then traverse if it is ignored. Forms
recheck input validators on submission as well as their own field validators.

Store optional UI-thread validator callbacks in Rc; non-Send UI state remains
legal. Validate after committed changes and final truncation. Completion uses the
same insertion/limit/validation path. Suggestion source/search strings are cached;
matching-index buffers are reused. Map lowercase prefixes back to original scalar
boundaries, accounting for expansions such as İ, rather than slicing UTF-8 by the
typed byte length. Empty selections are Option values, never negative indices.

EchoMode supports normal, masked and hidden display. Word operations in masked
modes use whole-prefix/suffix movement/deletion to avoid revealing word breaks.
Hidden mode keeps the display cursor fixed. Debug output redacts masked values;
this is display behavior, not a memory-erasure guarantee. Normal/placeholder views
clip atomic graphemes by cells; reverse accumulation replaces quadratic suffix
width scans. Construct only visible text/masks. Reserve cursor width for a wide
completion. A leading combining completion is accepted but not previewed as a
separate cursor glyph. Width Some(0) hides the field; None removes its constraint.
The default empty prompt preserves Matea examples; applications can supply one.

Still pending: OS clipboard requests, real cursor hints/shape/color, virtual blink
lifecycle, default light-theme helpers and complete backend associated-text/IME
support. These stay explicit partial capabilities. The current Component returns
typed state actions and never owns terminal I/O or starts hidden timers.

Fixed source: Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4,
textinput/textinput.go, styles.go, textinput_test.go and internal/runeutil/runeutil.go.
See the input editing audit for executed edge cases, issue analysis and limits.
