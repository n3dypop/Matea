# ADR 0011: Preserve input events and own each terminal mode independently

Status: implementation decision; provisional public API, not frozen.

The original adapter discarded releases, function/media/modifier keys and mouse
events, and retained only three modifiers. Focus handling also accepted modified
Tab and release events would have repeated widget actions if exposed unchanged.

Move transport-neutral input values into core::input. Key contains a typed code,
six modifier bits, press/repeat/release kind and keypad/lock state. Copyable bitflags
avoid allocation and impossible combinations of duplicate modifier representations.
Key::new remains a plain press. Built-in widgets and bindings match down events
(press or repeat) and ignore releases; applications can inspect every event.
Chord matching ignores transient event kind/lock state, preserving autorepeat.
Focus traversal accepts only Tab/Shift-Tab/BackTab, without command modifiers.
Ctrl-C defaults to exiting only on down events with Control (Shift permitted,
other command modifiers excluded); CtrlCBehavior::Deliver routes it to the app.
NUL from a backend
maps to KeyCode::Null rather than text; control characters cannot be inserted by
the single-line widget's character-key path.

The terminal adapter exhaustively translates released Crossterm 0.29 Event/KeyCode/
MediaKeyCode/ModifierKeyCode/MouseEventKind variants. It does not promise physical
key identities, alternate key codes, associated text, pixel mouse coordinates or
extra mouse buttons that this backend does not expose. The pinned Git source is
newer than the dependency; do not claim complete pinned-repository parity.

InputOptions adds opt-in mouse capture, configurable paste/focus reporting and an
explicit Kitty keyboard request. Default behavior remains legacy keyboard, no
mouse, paste/focus enabled. No environment-based keyboard-support claim is made.
The example requests alternate keys as well as event kinds and all-key reporting.
Crossterm substitutes the layout-provided shifted character and consumes SHIFT;
Matea preserves that backend result, not a physical-key identity. Without alternate
key data, do not guess layout-dependent shifted characters from an unshifted code.
Do not call Crossterm's blocking enhancement query during startup: pinned
terminal/sys/unix.rs:154 waits unboundedly for DA1 after receiving CSI ? flags u
([report #1133](https://github.com/crossterm-rs/crossterm/issues/1133)). Bounded active
negotiation remains pending. Explicit Kitty mode is a caller assertion of support;
the legacy Windows command backend rejects it before acquiring terminal modes.

Track alternate-screen/cursor/style/paste/focus/mouse/keyboard leases individually and
clear each successful restoration independently. Pop keyboard flags before leaving
the alternate screen. If one restore fails, retry only remaining leases, avoiding
a second pop after an unrelated failure. Terminal write failures can be partial;
restoration is best effort and retains failed leases for retry, not a guarantee of
remote terminal state. Continue restoring other independent modes on errors.

Source audit (fixed revisions in upstream-lock.json):

- Crossterm event.rs:278, 816, 913, 977, 999, 1017, 1158, 1210, 1306.
- Ultraviolet key.go:15–45 and 280–310; Bubble Tea key.go/mouse.go.
- [Kitty keyboard specification](https://sw.kovidgoyal.net/kitty/keyboard-protocol/),
  read 2026-10-16: separate main/alternate stacks, push/pop ordering and event types.

Migration: Key struct literals use modifiers/kind/state; prefer Key::new builders.
Use key.modifiers.contains(Modifiers::CONTROL) instead of key.ctrl. App key-code
matches must check key.is_down() when they should not react to release.
