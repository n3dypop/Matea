# ADR 0040: Align logical time before input creates subscriptions

Status: accepted for the provisional event-loop contract; validation is tracked
with the deadline slice.

A new independent backend test reproduces an early notification: wait 10ms for
input, let that input create a 2ms subscription, then advance the runtime by the
entire previous wait. The new subscription fires before its own requested wait.
The failing test is retained in `docs/evidence/deadlines/input-start-before-failure.log`.

After polling, handle an interrupt or the configured Ctrl-C quit immediately.
Otherwise synchronize the runtime's logical clock and process already due
subscriptions before dispatching the incoming input/resize. A subscription
created by that input therefore starts at the updated logical time. Preserve
coalescing, cancellation, terminal restoration and the existing completion drain.

This explicitly orders already expired timers ahead of ordinary input at that
poll boundary, with interrupts first and worker completions later. Bubble Tea's
channel selection does not supply an identical deterministic ordering; this is
Matea's documented contract, not a claim of reproducing every interleaving.
No additional public time-mutating API or background input thread is introduced.

The regression checks that a second poll is necessary after starting the new
subscription. It uses a real bounded sleep to make the old logical-time error
observable; it does not assert a wall-clock upper bound. Deadline precision,
worker wakeups and longer blocking application callbacks remain separate limits.
