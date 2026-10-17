# ADR 0020: Validated spinner animations and borrowed frame access

Status: implementation decision; API provisional, module parity pending.

Replace the fixed four-frame component with a validated SpinnerAnimation and a
separate non-Clone Spinner state. Provide the twelve pinned Bubbles presets as
immutable static frame data, and share custom owned frames through Arc. An empty
sequence is invalid; an empty individual frame is valid (Ellipsis). TickInterval
excludes zero periods. Keep preset attribution and the full source MIT notice.

Reuse Stopwatch's monotonic time, identity, generation and UI-thread schedule
adapter. Start is idempotent; pause records partial intervals; reset/configuration
changes invalidate old schedules. Derive the frame from total active elapsed time
so delayed/coalesced ticks skip obsolete frames without an unbounded catch-up loop.
Do not advance a frame just because a duplicate/foreign/stale tick arrived. Keep
the existing explicit tick(theme) operation for owner-controlled manual cadence.
Do not Clone live state or interpret an absent ID as broadcast.

Expose borrowed frame_text separately from structured styled views. Optional
labels normalize trailing ASCII spaces at the boundary and use one separator; an
empty frame shows the label alone.
Reduced-motion themes suppress schedules and display a stable indicator; callers
can omit schedules while hidden without teaching the Runtime about spinners.
Theme accent remains the default, with an explicit Style override and ordinary
View composition. Full Lip Gloss decorators remain part of the wider style gap.

Frame selection and plain rendering benchmarks must distinguish borrowed output,
fresh owned strings and actual timer scheduling. Go Update creates a real timer;
comparing it to a Rust descriptor is not an equivalent active-update workload.
Keep active scheduling/notification performance pending until matched evidence.

Reference: Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4,
spinner/spinner.go and spinner/spinner_test.go. Retain executed edge observations
separately from issue reports. Public source Model.Style/WithStyle remains partial
while the framework's rich style surface is incomplete.
