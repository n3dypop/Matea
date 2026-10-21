# Clock reference-surface review

The existing Stopwatch and Countdown implementation now has a finite reference
assessment of 75. This reviews the two pinned Bubbles clock packages and their
functional correspondence, with the deliberate behavior changes in
[ADR 0019](decisions/0019-monotonic-clocks-and-generation-safe-schedules.md).
It does not change the package scope or the threshold. Performance remains 50.

The raw source inventory contains 21 stopwatch and 22 timer declarations. An
original compiler-export probe enumerates exported roots, addressable promoted
fields, and both value/pointer method sets. Its reachable signature walk checks
for private named exposure, generics and nonexported interface requirements.
Linux amd64, macOS arm64 and Windows amd64 all produce exactly the same 43
canonical members as the pinned inventory. Timer Start/Stop/Toggle are correctly
pointer-only; stopwatch controls are callable through values and pointers. No
additional own-package field or method is discovered in these two packages.

[Exact member signatures, IDs, receivers and hashes](evidence/clocks-exports/verification.json)
bind each member to the existing implemented capability and current code. The
selected source is one unconditional file per package and matches the pinned
Bubbles commit. Cmd/Msg/Event/Duration are explicit external signature boundaries;
the Cmd function, Msg alias, empty Event interface and duration primitive shapes
are reviewed. This is not the whole external method API or dependency frontier.
The project-wide Go target/export gate stays false.

| Source surface | Rust correspondence and existing validation |
| --- | --- |
| Model/New/Option/WithInterval/Interval | Constructors, TickInterval and setters; defaults, nonzero checks, interval changes and Runtime reconciliation |
| IDs on models/messages | Opaque ClockId getters, identity-bearing schedules/ticks; concurrent uniqueness and foreign-token regressions |
| Init/Start/Stop/Toggle/StartStopMsg | Mutable controls plus scheduling intent/subscription composition; pause, idempotent start, toggle and Runtime tests |
| Reset/ResetMsg/Elapsed | Reset/elapsed and projected elapsed; partial intervals, reset and stale-generation regressions |
| Timeout/Running/Timedout | Cached/projected remaining time, retiming and status; expiration, pause/rearm and zero tests |
| TickMsg/Update | Owned identity/generation/time tokens; duplicate, backward, stale and foreign events plus delayed Runtime delivery |
| TickMsg.Timeout/TimeoutMsg | Exactly-once ClockUpdate::TimedOut with identity; expiration and control-triggered timeout tests |
| View | ClockText/Display, structured themed view and caller formatting; 269 pinned strings, Rust maximum duration, Unicode units and style tests |

The same six program/fixture paths match the exact source of
[04e81bc CI](https://github.com/n3dypop/Matea/actions/runs/39495143723). Native
tests execute on all three OSes. Each reference-probe job separately rechecks the
source clock-edge observations, all 269 duration strings and sixteen existing
clock output cases. Raw clock/duration output is not uploaded separately in this
artifact set; retained checker output hashes and the exact checker source bind
these claims. Compiler profiles are separate from native execution evidence.

The corrected zero default, nonnegative expiry clamp, observed elapsed time,
generation filtering and direct timeout outcome intentionally differ from source
bugs and command ordering. Existing regressions and source observations in the
[clock audit](clocks-audit.md) remain unchanged; this is functional Rust
correspondence, not literal reproduction of those defects. The eight maintained
clock integration tests cover the stated contract. No new production code or
timing samples were needed for this assessment.

After the assessment input changes, the [local verification](evidence/clocks-exports/native/metrics.json)
again passes 47 commands, Rust 277 plus 11 doctests, Python 146 with two
conditional skips, and LLVM 95 source paths/102 compilation instances. The three
transitive hashbrown zero-hash profile warnings remain retained and diagnosed.
Selected native logs and reports are archived; the qualification Git checkpoint
retains all raw command logs. Execution coverage is not a branch or safety proof.

The existing runtime subscription cap, unbounded application composition, OS
wakeup precision and other target profiles remain limits. Active scheduling,
cancellation/notification latency, variable-duration workloads and retained
memory still need matched measurements. Prior failed formatter candidates and
adverse samples remain retained; no maximum-performance or complete Foundation
closure claim follows from this review.
