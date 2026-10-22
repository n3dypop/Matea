# Spinner capability audit

Reference: Bubbles `c3bde0d166c2ca607d9864493453fd77bdcae7e4`,
`spinner/spinner.go` (29 declaration candidates) and `spinner/spinner_test.go`.
The source group has no platform/build-tag conditions. All candidates are linked;
**full style and module performance coverage remains pending**.
[ADR 0020](decisions/0020-validated-spinner-animations.md).

| Source declarations | Matea implementation and limits |
| --- | --- |
| Spinner, Frames, FPS | immutable SpinnerAnimation, shared custom strings, nonempty sequence and positive TickInterval |
| Line, Dot, MiniDot, Jump, Pulse, Points, Globe, Moon, Monkey, Meter, Hamburger, Ellipsis | twelve SpinnerPreset variants; all 66 frames and integer-divided periods checked against executed source fixtures |
| Model, Model.Spinner, New, Option, WithSpinner | non-Clone Spinner, new/default/new_at, animation getter and transactional setter; set_animation resets phase and invalidates earlier ticks |
| ID, TickMsg, Time, ID field, Tick, Update | shared opaque ClockId, generation/time-bearing ClockTick, Stopwatch schedule adapter, filtered updates and explicit manual tick; no ID-zero broadcast |
| View | borrowed frame_text/Cow text, optional labeled text, styled structured Component view |
| Model.Style, WithStyle | basic framework Style override and ordinary View composition; arbitrary Lip Gloss decorators are **partial** |

Preset frame data/periods carry the full pinned Bubbles MIT attribution in
[THIRD_PARTY_NOTICES.md](../crates/matea-widgets/THIRD_PARTY_NOTICES.md). No reference
implementation bodies are vendored. This notice does not select Matea's overall
distribution license.

## Executed source observations

[spinner-source-edges.json](evidence/spinner-source-edges.json) pins the harness,
executable and fixture hashes. The harness uses public constructors, Tick, Update
and fields, including one real positive-tag timer callback. It does not mutate
private fields or run an infinite zero-period program.

- Replaying the initial Tick advances Line from `/` to `-`: its zero tag bypasses
  the positive-tag guard. A later stale **positive** tag is correctly rejected by
  source. Matea rejects duplicate timestamps and old generations from the outset.
- A public zero-ID TickMsg advances a source spinner. Matea's ticks require an
  actual opaque identity. Explicit manual tick is the owner-controlled alternative.
- Empty source frame sequences and a shortened sequence at the current index
  produce `(error)`. Matea rejects empty construction and resets the phase when
  replacing data. Empty individual frames remain valid for Ellipsis/custom data.
- Source accepts a zero FPS period. Matea reuses positive TickInterval; no spin or
  CPU-load measurement is inferred from the source constructor observation.

[#999](https://github.com/charmbracelet/bubbles/issues/999) was filed in Bubbles but
its code/description concerns **Huh** spinner title concatenation. Inspection of
pinned Huh `ffb6a97195b4a031925a31cc70c62fc1d309e69d`,
`spinner/spinner.go` View confirms the direct `spinner.View() + title` composition.
No Huh runtime reproduction is claimed. Matea's optional label trims trailing
ASCII spaces at that boundary and inserts one separator for nonempty frames;
an empty frame shows only the label. Line, Dot and Ellipsis have regressions.
Broader Huh spinner action/accessibility/lifecycle capabilities remain pending.

## Rust behavior and validation

Live state has one identity and cannot be Clone. Immutable custom animation data
clones through Arc; presets borrow static strings. Unlabeled text borrows a frame
through Cow, while adding a label allocates a composed string. Structured Views
own text and follow the compositor's Unicode/clipping/sanitization contracts.

Use total active monotonic elapsed time divided by the period to select a phase.
Late updates skip obsolete frames in bounded work; pause preserves partial periods.
Controls/configuration changes invalidate old schedules. Repeated start is
idempotent. Hidden/reduced-motion schedules can be omitted; when a still-running
clock returns it skips forward to the current phase. Reduced-motion rendering is
a stable `*`; explicit manual cadence also respects the supplied theme.

Five integration tests cover the source data, custom validation, independent IDs,
manual cadence, labels/style, delayed/duplicate/foreign/stale/backward updates,
partial pause, configuration reset and actual Runtime reconciliation. The latter
checks coalescing and removal/recreation of paused/reduced-motion subscriptions
with an injected monotonic clock and no sleeps. The `spinners` example displays
all twelve presets; snapshot and Unix PTY checks cover controls and restoration.
These tests do not prove real emulator glyph widths or wake precision.

Fourteen finite performance cases cover the twelve plain frame strings and two
ignored-tick guards. Storage sharing is allowed in both languages: Rust Cow and
Go's unstyled View can borrow immutable frame storage. They do not require a
fresh owned copy. Active Go Update creates a real timer, so comparing it to
Matea's schedule descriptor would be unequal. Active scheduling/notification,
custom construction/reconfiguration, labels and rich styled Views remain pending.
See [performance evidence](performance.md); these cases are not module parity.

In the current one-host run, the twelve plain first-frame cases have p50
2.19–2.27 ns in Matea and 636.05–644.64 ns in source; both allocate zero. The two
ignored-tick-plus-frame cases have p50 4.07/4.24 ns versus 723.98/715.66 ns. Source
Model.View still executes default Lip Gloss Style.Render; Matea's plain text path
borrows directly. The latter timings do not isolate generation/ID guard speed.
They exclude styled View construction, cell composition and terminal output.
