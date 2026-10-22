# Unicode input editing and completion audit

Bubbles pin: `c3bde0d166c2ca607d9864493453fd77bdcae7e4`. Source:
textinput/textinput.go, styles.go and textinput_test.go. The 77 declaration
candidates are split between implemented editing/suggestions and partial
presentation/lifecycle contracts in the capability ledger. Full TextInput module
parity remains pending. Design: [ADR 0013](decisions/0013-grapheme-input-and-completion.md).

## Executed source findings

[Raw observations and source pins](evidence/textinput-source-edges.json) come from
the fixed Go reference through the original comparison harness, without a terminal
session or clipboard read. `probe --textinput-source-edges` reproduces them.

- [Bubbles #1056](https://github.com/charmbracelet/bubbles/issues/1056): Alt+D at
  position 1 yields `a d` from `abc d`, incorrectly yields `a` from `ab c`, and
  panics on `ab`. The pinned source reproduces all three. Matea deletes to the next
  whitespace boundary and returns `a d`, `a c`, `a`; regression tests also assert
  valid global grapheme/cursor state.
- Source completion appends outside the normal insertion path. With value t,
  suggestion tea and CharLimit=2, the result is tea and the validator call count
  remains unchanged. Matea yields te, reruns validation and updates its error.
- SetValue does not refresh the source matched suggestions. Matea updates the
  cache on programmatic values as well as edits and preserves selection only if
  the matching index list remains the same.
- Empty previous-suggestion followed by CurrentSuggestion panics in the source;
  Matea uses an absent selection and ignores cycling when no matches exist.
- The source's Chinese placeholder contains NULs because display columns index a
  rune buffer. Matea iterates graphemes and clips by cells, with no NUL padding.
  Both skipped placeholder tests (source lines 52/65) are assessed with limits:
  Matea tests deterministic wide/long/empty clipping, not the skipped expected
  string's exact padding/ellipsis policy. Text is clipped rather than adding an
  implicit ellipsis.
- [Bubbles #950](https://github.com/charmbracelet/bubbles/issues/950): the pinned
  width-0 placeholder is only `> S` for Something. Matea uses set_width(None) for
  unconstrained content and Some(0) for an explicitly hidden field. Tests cover
  both. This is a typed API correction, not identical integer-zero semantics.
- [Bubbles #562](https://github.com/charmbracelet/bubbles/issues/562) describes
  an older v1 down-arrow crash. The pinned-v2 adaptation (foo/bar, f, ten Down
  events, View/current selection) does not panic. Do not call the distinct empty
  previous-selection reproduction the same historical crash.
- [Bubbles #906](https://github.com/charmbracelet/bubbles/issues/906): the pure
  native cursor getter reports X=3 for あいう instead of six columns in the fixed
  source. Matea now uses the same grapheme-cell viewport for virtual and native
  cursors. Native placement, prompt offsets and hidden/password echo have regressions.
- [Bubbles #1001](https://github.com/charmbracelet/bubbles/issues/1001): an adapted
  scroll reproduction uses 30 ASCII letters, width 20, prompt `> ` and one Left
  after CursorEnd. The pinned native getter returns column 22 while the scrolled
  content needs column 21. The source's getter uses absolute rune position rather
  than the viewport offset. Matea returns 21. The report's end-position assertion
  does not subtract prompt width, so the executed Left case is the clearer witness.

## Rust behavior and evidence

The component edits a String in place. Ignored events do not clone it or reset
validation/submission state; character input uses stack UTF-8 encoding. Cached
ASCII/count state and borrowed iterators avoid boundary vectors. Rune/scalar and
grapheme indices deliberately differ. Tests cover combining accents, emoji ZWJ,
regional indicators that merge after deleting a separator, cap-aware pastes that
preserve the old suffix, byte limits, control sanitization, case expansion, masks,
custom/disabled keys, focus/release/repeat and custom styles.

Validation runs after actual edits/truncation. Form submission rechecks callbacks
that capture other UI state; child validation errors cannot be bypassed by the
outer required-field validator. Tab completes before focus traversal, then moves
focus when there is nothing left to accept. The form example exercises this path
through the real Crossterm reader in a Unix PTY.

Native input uses `set_virtual_cursor(false)` and a local `View::with_cursor`
hint. The compositor translates it through rows, columns, panels and layers;
clipping or later opaque paint can hide it. `Surface::cursor` is frame-relative.
Cursor-only updates work without cell damage. The encoder hides the cursor while
painting and places it after the batch. `form --native` exercises CJK paste and
restoration in a Unix PTY. This verifies emitted coordinates, not an actual
emulator's width implementation. Shape/color, blinking, real IME and native Windows
console behavior remain pending. Design: [ADR 0014](decisions/0014-composed-cursor-and-pure-encoding.md).

Eight matched performance cases separate six ASCII editing/navigation pairs, a
Unicode whitespace-word roundtrip and active suggestion filtering. Virtual cursor
effects, View construction, OS clipboard and validators are disabled in both timed
harnesses. Unicode single-character movement/deletion is not compared because the
source's rune operations and Matea's whole-grapheme operations differ. The Unicode
word case checks the normalized end position outside timing. Limits do not bind
the small common input fixtures. Construction is not timed; buffers are warmed.

Theme styles, echo and native position/visibility are functional, but cursor
appearance/blink/clipboard/light-default APIs, rich associated text and real IME tests
remain pending. Suggestion matching uses Unicode lowercase prefixes, not full
case folding, normalization or fuzzy matching. No full-module or universal latency
claim is made by the finite workloads in [performance.md](performance.md).
