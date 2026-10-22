# Input event preservation and terminal mode recovery

Status: implemented input slice; full reference/module parity remains pending.
Design and migration: [ADR 0011](decisions/0011-input-events-and-mode-leases.md).

## Source scope

Crossterm `9a397df6dc4f5bd3ff900bb6308b728186cb914c` `src/event.rs` defines
key codes, six modifiers, keypad/lock state, three key kinds and mouse events.
The dependency is released 0.29.0, not that newer Git checkout. The adapter now
exhaustively converts the released enum variants, preserves their state and moves
paste strings without cloning. NUL becomes a dedicated Null key. Input types in
core do not depend on Crossterm. The Git checkout's new helper methods and macros
are still pending in the full API ledger.

Bubble Tea `f9df43c4f2c021d8740a2628d59cd810a862eb98` key.go/mouse.go and
Ultraviolet `6b8d4baf91b4c933e6613b0eefc8f180f6eda002` key.go/event.go expose
press/release, repeat/modifiers and richer physical/associated-text fields. Those
fields and extra mouse buttons/pixel coordinates cannot be reconstructed from
Crossterm's public event data. They remain pending, not silently synthesized.
For Kitty alternate-key sequences, Crossterm 0.29.0
`src/event/sys/unix/parse.rs:598` substitutes the layout-provided shifted character
and consumes SHIFT. The inspector requests REPORT_ALTERNATE_KEYS and PTY tests
cover both A and a shifted £ symbol. Matea preserves the backend value; without
alternate data it cannot recover layout-specific text or the original physical key.

Bubbles `c3bde0d166c2ca607d9864493453fd77bdcae7e4` key/key.go:147 formats the
event to a string before testing binding state. Matea matches typed code/modifier
values, skips disabled/unbound bindings and rejects release once before scanning
shortcuts. Public fields replace option/setter wrappers; unbind clears shortcuts
and help together. Arbitrary fmt.Stringer and associated-text chords require
further capability review. Six matched workloads are recorded in performance.md.

## Applicable defects and improvements

- [Bubble Tea #1865](https://github.com/charmbracelet/bubbletea/issues/1865)
  reports Windows modifier key-down arriving as a NUL character. Matea normalizes
  backend NUL to Null and its single-line widget does not insert control-valued
  character events. Synthetic conversion/widget tests cover that boundary. This
  does **not** fix or reproduce the native Windows parser's classification; native
  Windows, AltGr, IME and layout-specific physical keys remain unverified.
- [Crossterm #1133](https://github.com/crossterm-rs/crossterm/issues/1133): pinned
  terminal/sys/unix.rs:154 performs an unbounded DA1 read after receiving keyboard
  flags. Matea does not invoke that blocking query. Explicit Kitty requests avoid
  this startup path; bounded active negotiation is pending. Neither the upstream
  hang nor a bounded-query fix is claimed to have been executed here.
- The previous Matea FocusRing accepted Ctrl/Alt-Tab; the new traversal policy
  accepts plain Tab, Shift-Tab and BackTab only. Built-in components ignore release
  events while preserving repeat. Ignored events no longer reset a Form's submitted
  state. Regression tests exercise command modifiers, locks and repeated keys.
- A single restoration flag could replay successful actions after an unrelated
  failure. The transport now owns independent style/cursor/paste/focus/mouse/screen/
  keyboard leases. Restoration retries only failed leases. A failed keyboard pop
  keeps the alternate screen active so the retry operates on the correct stack.
  Portable mocks inject every acquisition/restoration failure. Partial I/O still
  makes remote terminal state uncertain; best-effort cleanup is not transactional.
- Ctrl-C key handling is configurable as Quit or Deliver. Release and chords with
  extra command modifiers are forwarded; OS termination signals remain cleanup
  events. Custom backends use the same tested driver policy.

## Validation and limits

The converter test covers 64 modifier masks × 8 state masks × 3 kinds, function
keys F1–F35, every released media/modifier/nonpayload code, four wheel directions,
buttons/drag/move, zero-based coordinates, paste ownership, focus and resize.
Widget tests verify releases cannot edit, navigate, submit or steal focus.

`cargo run -p matea --example input` is a runnable inspector. `--mouse` requests
capture; `--kitty` is an explicit compatible-terminal override, and
`--deliver-ctrl-c` leaves that key to the application. The current native Windows
backend rejects Kitty requests before acquiring terminal state; legacy Windows
events can still carry press/repeat/release data when supplied by Crossterm.

Unix PTY tests send F12, CSI-u repeat/release/shifted text, Super, Ctrl-C release/delivery, SGR
mouse, bracketed paste and focus changes through the real Crossterm reader. They
also verify a single keyboard pop precedes alternate-screen exit and mouse capture
is disabled. A PTY byte test does not certify Kitty or any real emulator version.
The input fixture is not an image renderer, codec or graphics engine.
