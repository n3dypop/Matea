# Virtual cursor source audit

Reference: Bubbles `c3bde0d166c2ca607d9864493453fd77bdcae7e4`,
[`cursor/cursor.go`](https://github.com/charmbracelet/bubbles/blob/c3bde0d166c2ca607d9864493453fd77bdcae7e4/cursor/cursor.go).
All 21 public declaration candidates are reviewed and linked. This source group
has no platform/build constraints. Rich styling and full module performance
remain partial; linking declarations does not close their equivalence review.

| Reference declarations | Matea API and evidence |
| --- | --- |
| Mode, CursorBlink, CursorStatic, CursorHide, Mode.String | CursorMode enum and Display; invalid numeric modes are unrepresentable |
| Model, New | non-Clone VirtualCursor, new/default/new_at; one exclusive identity |
| Model.Style, Model.TextStyle, View | optional framework styles and structured view; reverse block/ordinary text phases tested; arbitrary Lip Gloss decorators remain partial |
| Model.BlinkSpeed | positive TickInterval with default 530 ms, transactional setter/getter |
| Model.IsBlinked, Mode, SetMode | read-only cached blink phase/mode and generation-invalidating mode control; hidden does not erase the underlying text |
| BlinkMsg, Update | opaque ClockTick, schedule, update; foreign, duplicate, old-generation and backward notifications rejected |
| method Blink, function Blink | owner-scoped restart/schedule/subscription; explicit focus/restart replaces the global initial broadcast |
| Focus, Blur | explicit controls and injected-time variants; reset phase and remove inactive schedules |
| SetChar | retained owned String with reusable capacity and borrowed text accessor |

[Executed source observations](evidence/cursor-source-edges.json) use public
constructors and actual timer callbacks, without reflection or upstream edits.
Default/focused/first-tick/static/hidden/blurred state and mode names match the
Rust fixtures. Source correctly rejects duplicate messages and cancels a prior
command on a new Focus. Its historical closure race is already fixed at this pin.
The expired-message mode-roundtrip defect is reproduced and prevented by
Matea's stronger generation boundary. Nonpositive source durations, raw numeric
modes and shared live model copies are deliberately excluded by Rust types.

Matea selects phase using total elapsed periods rather than delivered-message
count. Reduced motion shows the focused cursor steadily and omits timer
subscriptions; hidden/blurred cursors remain hidden. Theme changes can reveal
the current monotonic phase when animation resumes. These are explicit behavior
improvements, not claims of identical scheduling internals.

`TextInput::view_with_cursor` borrows this state; text models can still be
cloned independently. The owner forwards focus and ticks and calls restart on
editing, as demonstrated by the `cursor` example. InputStyles controls input
colors. Password, hidden echo, suggestions, combining characters, CJK, emoji,
native cursor hints, clipping and escape sanitization have integration checks.

Seven optional `--virtual-cursor` cases compare fresh 16x1 grapheme cells plus
owned plain text/reverse flags, nonblinking focus/blur, and ignored foreign
notifications. Both languages return exactly the same semantics, output and
first value. The original 84-case cohort remains unchanged. No active Go timer
is compared to a Rust descriptor. Basic cell rendering is not arbitrary style
equivalence, and control comparisons include Matea's stronger time/epoch guards.
See [ADR 0035](decisions/0035-owned-virtual-cursor-and-expiring-notifications.md).
