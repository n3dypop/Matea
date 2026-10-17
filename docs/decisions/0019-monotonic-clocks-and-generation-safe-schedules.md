# ADR 0019: Monotonic clocks and generation-safe schedules

Status: implementation decision; provisional API and finite module review pending.

Implement stopwatch/countdown state in widgets, using std::time::Instant with
explicit _at methods for deterministic tests. Values advance by observed elapsed
time, not by counting nominal tick intervals. Pause captures the partial interval;
stopped time is excluded. Countdown clamps at zero and reports timeout in the same
filtered update, exactly once. UI methods directly mutate &mut state, avoiding
Go's copied-model/control-command ordering issue. No extra worker per clock.

Clock identity is opaque and unique across both types. A Copy ClockSchedule carries
identity, generation and nonzero interval; ticks carry identity/generation/time.
Controls, reset/retiming and interval changes invalidate earlier schedules. Older,
duplicate, foreign and out-of-order ticks are ignored. A direct advance_at supports
owner-authorized broadcast time without a magic zero ID. Backward control times
leave state unchanged; elapsed Duration saturates on overflow. IDs/generations fail
loudly rather than wrap after u64 exhaustion, beyond practical clock lifetimes.
Models are not Clone; copying a schedule intentionally retains its generation.
Explicit reset/retiming cancels the prior pending expiration. Callers inspect
timeout outcomes on controls as well as ordinary tick updates. Cached accessors
describe the last accepted observation; projection accessors do not notify/mutate.

Use existing UI-thread subscriptions. Schedule IDs include identity, generation
and effective interval so reconciliation retains an unchanged closure and replaces
a changed deadline. The last countdown interval is shortened to the remaining
duration. The adapter captures only schedule data, never widget state. An injected
clock enables Runtime tests without sleeping; ordinary controls and subscriptions
use the same system Instant domain. Subscription construction still allocates under
the current Runtime contract; a cheap schedule/update does not prove end-to-end
zero allocation or real terminal latency.
Driver polling and application Update/View work still bound notification latency;
elapsed-value correctness does not guarantee a real-time wakeup. Runtime logical
time saturates at Duration::MAX: exact terminal deadlines fire once, and checked
addition represents unrepresentable future deadlines as None instead of repeatedly
rounding them down to the same maximum timestamp. A dedicated regression covers
both creation overflow and rescheduling at the maximum.

TickInterval rejects zero; unsigned Duration excludes negative intervals. Defaults
are one second, correcting the pinned stopwatch constructor's zero default.
Countdown setters preserve pause and require explicit start after expiration.
Zero duration is already timed out without a synthetic constructor notification.
Returned ClockUpdate::TimedOut replaces an extra asynchronously ordered TimeoutMsg;
the caller can translate it into any domain message. Expose cached values, pure
projection at a timestamp, compact Display and ordinary themed views. Formatting
is independent of scheduling, permitting custom application formats.

Reference: pinned Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4,
stopwatch/stopwatch.go and timer/timer.go. Record executed default/stale/start/timeout
observations separately from issue reports. Regular-cadence performance fixtures
must match state/output/scheduling intent; intentional late-tick, partial-pause,
zero/negative and notification differences need regressions, not parity claims.

Format nonnegative Duration into a bounded safe UTF-8 stack buffer, with one exact
owned String allocation or caller-buffer Display reuse. Keep the zero fast path.
The finite Go formatting fixtures match, but measured fresh-string performance
remains slower in all six shared cases. Reused-buffer allocation is zero in both
languages. Preserve before/intermediate/final samples and keep module performance
review pending; ignored-update microbenchmarks do not cover active scheduling.
