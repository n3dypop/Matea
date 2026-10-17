# ADR 0035: Give virtual cursor notifications one owner and a monotonic phase

Status: accepted for the provisional Foundation component API.

Pinned Bubbles `c3bde0d166c2ca607d9864493453fd77bdcae7e4`,
`cursor/cursor.go`, exposes blinking/static/hidden modes, focus, a blink period,
text/styles and timer messages. Its value model shares a pointer to a mutable
cancel context. The pinned `cursor_test.go` already covers a historical closure
race; do not claim that race is still present at this revision. An executed
public-API fixture does reproduce another boundary: after a blink deadline,
switching static then blinking leaves the old tag valid until the initial
broadcast is handled, so an expired old notification can hide the newly shown
cursor. Preserve the actual observations in `cursor-source-edges.json`.

Use a non-Clone `VirtualCursor` with the existing exclusive clock identity,
generation and positive `TickInterval`. Every focus, blur, mode/period change
and explicit editing restart invalidates old notifications, even at the same
timestamp or with an unchanged setting. Reject backward-time controls before
mutating any cursor state. Compute blink phase from elapsed monotonic periods:
coalesced delivery skips obsolete toggles in constant work. The owner explicitly
forwards controls and reconciles subscriptions; no global initialization
broadcast, shared mutable context or private runtime change is required.

Controls do not allocate timer resources. `ClockSchedule` is a descriptor;
converting it into a runtime `Subscription` still allocates. Do not compare
descriptor construction with a Go command that allocates a real timed context.
Reduced motion omits schedules and shows a stable focused cursor. Static and
hidden modes do not schedule. Queued notifications after blur/mode changes are
rejected; changing only a theme does not itself change the clock generation.

Keep `TextInput` clonable and its existing default rendering behavior. Add
`view_with_cursor` to borrow separately owned animation state; `InputStyles`
continues to own input cell styles, and native cursor hints keep their existing
contract. Standalone cursor rendering uses normal structured Views and their
Unicode, clipping, sanitization and terminal fallback. Arbitrary Lip Gloss
decorators are still part of the unresolved shared style capability, not
silently counted as complete.

Five independent integration fixtures cover mode/focus/style combinations,
phase/coalescing, all stale-control paths, text/password/suggestion composition
and real Runtime reconciliation with an injected clock. The executable `cursor`
example supplies editing restarts and focus handling; snapshot and Unix PTY
checks exercise authoring and terminal restoration. Source timer fixtures and
seven matched finite render/control workloads provide bounded evidence; full
timer delivery, resident memory and module performance remain open.
