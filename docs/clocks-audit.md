# Stopwatch and countdown capability audit

Reference: Bubbles `c3bde0d166c2ca607d9864493453fd77bdcae7e4`,
`stopwatch/stopwatch.go` (21 public declaration candidates) and `timer/timer.go`
(22). Both groups are unconditional Go APIs; their dependencies are sync/atomic,
time and Bubble Tea commands. All 43 candidates are linked to implementation and
correctness evidence. **Module performance remains pending**, including active
scheduling and the remaining formatter gap. [ADR 0019](decisions/0019-monotonic-clocks-and-generation-safe-schedules.md).

| Source declarations / capability | Matea contract |
| --- | --- |
| Model, New, Option, WithInterval, Interval | Stopwatch / Countdown, constructors, positive TickInterval, getters and setters; one-second defaults |
| ID and ID fields on messages | opaque ClockId, unique across both types; exposed through clocks, schedules and ticks |
| Init | start the paused stopwatch and expose a schedule; positive countdowns already expose a schedule; parent Model returns subscriptions |
| Start, Stop, Toggle, StartStopMsg | immediate mutable UI controls; system-clock helpers and explicit _at variants; no copied-model command ordering |
| Stopwatch Reset, ResetMsg, Elapsed | reset preserves running/pause, invalidates pending ticks; cached elapsed and nonmutating elapsed_at projection |
| Timer Timeout, Running, Timedout | cached remaining, remaining_at projection, explicit setter and flags; zero clamps/stops; expired clocks require start after retiming |
| TickMsg, Update | generation/identity/time-bearing ClockTick, filtered update, owner-authorized direct advance_at for broadcast time |
| TickMsg.Timeout, TimeoutMsg | ClockUpdate::TimedOut(id) at the accepted expiration, including control-triggered expiration, once; caller maps to a domain message |
| View | compact ClockText, exact-size owned string, Display, themed structured view; Duration getters allow application formatting |

None of these widgets acquires a terminal or spawns a worker. Schedule adapters
use the existing UI-thread Runtime. The descriptor is cheap/Copy; constructing a
subscription still allocates a callback/ID under the current Runtime contract.
Schedule IDs include generation and effective period so interval changes and the
short final countdown period replace the old subscription. Unchanged behavior
retains the existing timer/closure. Core's 64-subscription limit still applies.

Use a single monotonic Instant domain for controls and subscriptions. Injected
clocks support deterministic Runtime tests. Cached accessors describe the last
accepted observation; projecting a value does not notify or mutate it. Backward
controls/ticks and duplicate timestamps are ignored. Reset/retiming cancels the
prior pending expiration. Polling and application work still affect notification
latency; this is not a real-time scheduling guarantee.

## Executed source findings and changes

[Source observations](evidence/clocks-source-edges.json) pin the harness hash and
record the following. Reflection only dispatches the private sequence slice
returned by public Start; private IDs/tags/fields are not mutated. No modified
reference implementation is used.

- [#862](https://github.com/charmbracelet/bubbles/issues/862): the default stopwatch
  interval is actually zero. Matea defaults to one second and rejects zero periods.
  An infinite source program was not run; no CPU-spin measurement is claimed.
- [#1007](https://github.com/charmbracelet/bubbles/issues/1007): the timer tag never
  advances in pinned source. A tick captured before pause/resume still reduces
  ten milliseconds to nine. Matea invalidates that generation on controls.
- The initial stopwatch tick's zero tag bypasses the later tag check: replaying
  the same actual tick changes elapsed from one to two milliseconds. Matea ignores
  identical/out-of-order times as well as stale generations.
- [#867](https://github.com/charmbracelet/bubbles/issues/867): two public starts
  create two timer callbacks, reducing five milliseconds to three. Matea start
  is idempotent; it retains one subscription rather than launching extra tick loops.
- A one-millisecond countdown with a two-millisecond interval becomes `-1ms` in
  pinned source. Matea stops at zero, shortens the final period and returns one
  filtered timeout outcome. Initial zero is already expired without a synthetic
  constructor timeout. Negative periods are unrepresentable in unsigned Duration.
- [#237](https://github.com/charmbracelet/bubbles/issues/237): source counts nominal
  periods, so delayed handling loses elapsed time. Injected three-second/late and
  coalesced Runtime tests verify actual elapsed accounting in Matea. The reporter's
  real-machine timing figures were not reproduced and are not Matea measurements.
- [#128](https://github.com/charmbracelet/bubbles/issues/128) is a formatting extension
  request. Duration and ClockText are separate from scheduling, allowing caller
  formatting. Calendar months require an explicit external calendar policy; no
  calendar arithmetic is implied by elapsed time.

Rust's ownership removes the source's copied-model control-command pitfall.
Non-Clone clock state keeps identities unique; Copy schedules intentionally retain
their generation. Atomic ID allocation is checked, and generation overflow fails
loudly after u64 exhaustion rather than reusing tokens. Elapsed values saturate at
Duration::MAX. Countdown setters preserve pause, with explicit rearming rather
than an implicit resurrection from writing a positive duration.

Core also now represents an unrepresentable future logical deadline as None.
Previously saturating addition at Duration::MAX could leave the next deadline
immediately due again without time advancing. The regression checks both an exact
maximum deadline firing once and an overflowing future deadline never firing early.

## Correctness and performance evidence

Eight clock integration tests cover pause/partial periods, reset/interval epochs,
duplicate/foreign/old messages, backward time, shortened final deadlines, exactly
once timeout, retiming, concurrent unique IDs and real Runtime reconciliation.
269 fixed Go duration strings check both Display and into_string, including units,
fractions, large hours and maximum signed Go duration. Rust Duration::MAX and UTF-8
microsecond units/style are also checked. The clock example demonstrates two
independent schedules, pause, reset/rearm and timeout counts; Unix PTY validates
live interaction and cleanup. These are not physical console or wake-precision tests.

Six fresh formatting cases, six caller-owned buffer cases and four ignored-event
guards have matched outputs. Stack preparation avoids growing intermediate strings;
the longest shared duration changes from three allocations to one (56 to 24
requested bytes), with retained p50 148.167 to 54.417 ns. The initial stack version
regressed zero from 26.959 to 39.292 ns; a zero fast path brings it to 22.625 ns.
All before/intermediate samples are retained under clock_format_optimization in
[the retained clock checkpoint](evidence/performance/2026-10-16/clocks-checkpoint-report.json).

At that clock checkpoint, fresh formatting remains slower than Go in these fixtures (for example maximum
shared duration p50 54.417 versus 32.084 ns). Reused buffers allocate zero in both
languages, but most Rust formatter timings still exceed source. Ignored typed
ticks allocate zero and have lower measured p50 in these four narrow guards.
Neither observation proves module parity. Requested Rust allocator bytes and Go
MemStats/tiny-allocation rounding are not physical-memory equivalence.

An isolated active-update comparison would be misleading: Go Update arms a real
time.Timer and returns commands; Rust updates state and exposes a reusable schedule
descriptor. Matched end-to-end scheduling/notification latency, subscription
construction/cancellation, variable-duration workloads and remaining format
optimization are pending. Do not compare these different isolated operations and
claim a speedup. Three-OS CI validates builds/logic, not performance across hosts.
