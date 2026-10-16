# Key help capability and source audit

Status: partial module. All 27 public declaration candidates in pinned Bubbles
`c3bde0d166c2ca607d9864493453fd77bdcae7e4`, `help/help.go`, were read and linked.
They are unconditional Go APIs. Rich per-field Lip Gloss decorators and their
styled layout/performance remain pending; ordinary help is implemented.
[ADR 0018](decisions/0018-borrowed-and-width-bounded-key-help.md).

| Source declarations | Matea contract |
| --- | --- |
| KeyMap, ShortHelp, FullHelp | borrowed HelpItem arrays and HelpMap short/full groups, including caller-defined key token bindings |
| Model, New, ShowAll, View | Help::default, show_all, view; explicit short_view/full_view and plain text methods |
| Update | passive event returns ignored; no effects/runtime state |
| ShortSeparator, FullSeparator, Ellipsis | configurable sanitized strings; complete-item/column fitting |
| SetWidth, Width | public Option<u16> width: None unbounded, Some(0) hidden; cell bounds apply before emission |
| ShortHelpView, FullHelpView | short/full text and structured views share the same layout; top alignment, disabled/unbound suppression |
| Styles and its seven fields, Model.Styles | HelpStyles has all seven roles plus independent short_gap/full_gap; color and supported attributes work in structured cells. Arbitrary padding, wrapping, borders and richer decorators remain pending with the style system. |
| DefaultStyles, DefaultDarkStyles, DefaultLightStyles | from_theme, dark, light and Default; original Matea tokens rather than copied gray values |

Applications borrow labels/descriptions rather than clone key vectors; conversion
from Binding<K> shares its actionable enabled predicate. Regenerate items per
view call after state changes. The help example expands with `?` and enables or
disables an action with `d`; a Unix PTY exercises both modes, suppression and exit.

## Source findings and regressions

The pinned shouldAddItem branch accepts an oversized item when its ellipsis cannot
fit. Executed [source observations](evidence/help-source-edges.json) show a four-cell
`k go` output at widths one and two. Matea rejects the item, fits a whole ellipsis
only when possible, and emits nothing at width zero. Tests cover every width from
zero to sixty, exact tail fit, empty/emoji tails and disabled groups. These are
executed source observations, not maintainer-confirmed bug claims.

[Bubbles #571](https://github.com/charmbracelet/bubbles/issues/571) describes the
unstyled literal space between key and description. The pinned short/full code
still emits that literal outside the key/description style. Matea's separate gap
roles apply background/underline to the corresponding cells in both modes.
The report was read and assessed; this does not claim all requested style behavior
or rich Lip Gloss configuration is already implemented.

Keys, descriptions, separators and ellipses are sanitized. Newlines become inline
spaces, tabs use four spaces and controls become replacement glyphs. Standalone
zero-width clusters are omitted. Combining and ZWJ clusters remain intact. Widths
are computed once per prepared fragment from the same independent grapheme-cell
contract as the compositor; whole-string shaping width can differ (for example,
Arabic lam/alef). A regression verifies the gap follows both compositor cells.
Actual emulator shaping/ambiguous-width behavior remains unverified.

## Finite performance evidence

Four matched cases generate fresh unstyled strings: unlimited short help,
40-column short help, 40-column full help and an 80-column Japanese/combining
short help. Existing metadata is reused in both; binding construction, structured
view encoding and terminal I/O are excluded. Ordinary outputs match byte-for-byte,
including full-view padding. Width/safety deviations are separate regressions.

Capacity hints reduce short-string growth allocations; full help reserves its
output from the prepared layout and sizes active-item vectors explicitly. Plain
output can also append into a caller-owned String through short_text_into; its
reuse path is not included in the matched fresh-string allocation claim.

The first capacity version added a separate zero-width safety scan and regressed
the Unicode fixture from p50 299.875 to 518.417 ns. Those raw samples are retained.
The final preparation computes and retains widths during the same grapheme scan,
with a direct ASCII path. See all before/intermediate/current p50/p95, bytes and
allocations under help_text_optimization in
[report.json](evidence/performance/2026-10-16/report.json). Keep timing regressions
and scheduling outliers; no samples are discarded. This one-host plain-string
comparison does not close the module's rich styling or full performance review.
