# ADR 0039: Respect subscription deadlines when waiting for terminal input

Status: accepted for the runtime/terminal scheduling contract; measurement and
cross-platform evidence are tracked separately.

The native driver currently waits up to 16ms for input before advancing logical
time and receiving timer notifications. An idle application with a 1ms/5ms
subscription can therefore receive its first notification around the polling
quantum, while a 50ms timer can wait roughly four quanta. This follows from the
current source; a recording-backend test now reproduces the 1ms subscription
being postponed by a 16ms input wait. The failing test and initial four native
latency distributions are retained in `docs/evidence/deadlines`. Do not claim this has
been observed on a physical terminal or that it is an upstream defect.

Expose the next active subscription's relative logical deadline without
allocating or reading a platform clock. An independent driver can use it to
choose an input wait. In the default terminal driver, cap the existing wait by
that deadline, subtracting time spent rendering since the last logical advance.
Preserve coalescing, independent subscription identity, overflowing deadlines,
error/panic restoration and the single-input-poller boundary.

Use injected logical time for correctness fixtures, plus a recording backend
that rejects the old 16ms wait for a shorter subscription. Capture committed
before/after end-to-end first-notification distributions with a backend that
honors its requested timeout. Keep all samples, allocation counters and output
checks. Scheduling/OS overshoot is measured, not hidden or asserted away.

This is an individual runtime/driver defect fix, not entry into Stage B. Worker
completion delivery and terminal input still lack a unified wakeable event
source; a 16ms upper bound for that path remains a separate limitation. Do not
claim hard real-time deadlines, timer/worker performance parity or complete
Bubble Tea runtime coverage from the new timeout selection.

The public provisional API is `Runtime::next_subscription_delay() ->
Option<Duration>`. Its fixed-size timer table has at most 64 entries; inspection
uses one safe borrowed pass and no owned result allocation. `Some(Duration::MAX)`
and an unrepresentable future deadline remain distinct. Independent logical
fixtures cover nearest deadlines, removal/restart, coalescing and the maximum
duration boundary. The transport helper covers time spent rendering, overdue
deadlines and the unchanged idle cap without sleeps.
